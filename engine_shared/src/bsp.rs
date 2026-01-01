//! BSP map loader.
//!
//! This module parses Valve BSP files (versions 19-21, Source Engine era).
//! It extracts geometry, entities, and metadata needed for server simulation
//! and client rendering.
//!
//! Reference: <https://developer.valvesoftware.com/wiki/Source_BSP_File_Format>
//!
//! # Usage
//! ```ignore
//! let bsp = BspMap::load("maps/de_dust2.bsp")?;
//! println!("Map: {} entities", bsp.entities.len());
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

use crate::math::Vec3;

/// BSP file magic number: "VBSP" in little-endian.
pub const BSP_MAGIC: u32 = 0x50534256; // "VBSP"

/// Supported BSP versions.
pub const BSP_VERSION_MIN: u32 = 19;
pub const BSP_VERSION_MAX: u32 = 21;

/// Number of lumps in a BSP file.
pub const HEADER_LUMPS: usize = 64;

/// Lump indices (subset of what we care about).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum LumpIndex {
    Entities = 0,
    Planes = 1,
    TexData = 2,
    Vertices = 3,
    Nodes = 5,
    TexInfo = 6,
    Faces = 7,
    Leaves = 10,
    Edges = 12,
    SurfEdges = 13,
    Models = 14,
    Brushes = 18,
    BrushSides = 19,
    GameLump = 35,
    PakFile = 40,
}

/// Lump descriptor from BSP header.
#[derive(Debug, Clone, Copy, Default)]
pub struct LumpEntry {
    pub offset: u32,
    pub length: u32,
    pub version: u32,
    pub fourcc: [u8; 4],
}

/// BSP file header.
#[derive(Debug, Clone)]
pub struct BspHeader {
    pub version: u32,
    pub lumps: [LumpEntry; HEADER_LUMPS],
    pub map_revision: u32,
}

/// A 3D plane.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Plane {
    pub normal: Vec3,
    pub dist: f32,
    pub plane_type: i32,
}

/// A vertex (just a position).
pub type Vertex = Vec3;

/// An edge: two vertex indices.
#[derive(Debug, Clone, Copy, Default)]
pub struct Edge {
    pub v: [u16; 2],
}

/// A face (polygon).
#[derive(Debug, Clone, Default)]
pub struct Face {
    pub plane_num: u16,
    pub side: u8,
    pub on_node: u8,
    pub first_edge: i32,
    pub num_edges: i16,
    pub tex_info: i16,
    pub disp_info: i16,
    pub surface_fog_volume_id: i16,
    pub styles: [u8; 4],
    pub light_ofs: i32,
    pub area: f32,
    pub lightmap_mins: [i32; 2],
    pub lightmap_size: [i32; 2],
    pub orig_face: i32,
    pub num_prims: u16,
    pub first_prim_id: u16,
    pub smoothing_groups: u32,
}

/// A brush (convex solid).
#[derive(Debug, Clone, Copy, Default)]
pub struct Brush {
    pub first_side: i32,
    pub num_sides: i32,
    pub contents: i32,
}

/// A brush side.
#[derive(Debug, Clone, Copy, Default)]
pub struct BrushSide {
    pub plane_num: u16,
    pub tex_info: i16,
    pub disp_info: i16,
    pub bevel: i16,
}

/// A model (world or brush entity bounding info).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub mins: Vec3,
    pub maxs: Vec3,
    pub origin: Vec3,
    pub head_node: i32,
    pub first_face: i32,
    pub num_faces: i32,
}

/// Parsed entity from the entity lump.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BspEntity {
    pub classname: String,
    pub properties: HashMap<String, String>,
}

impl BspEntity {
    /// Gets a property value.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.properties.get(key).map(|s| s.as_str())
    }

    /// Parses the "origin" property as Vec3.
    pub fn origin(&self) -> Option<Vec3> {
        let s = self.get("origin")?;
        let parts: Vec<f32> = s
            .split_whitespace()
            .filter_map(|p| p.parse().ok())
            .collect();
        if parts.len() == 3 {
            Some(Vec3::new(parts[0], parts[1], parts[2]))
        } else {
            None
        }
    }

    /// Parses the "angles" property as Vec3 (pitch, yaw, roll).
    pub fn angles(&self) -> Option<Vec3> {
        let s = self.get("angles")?;
        let parts: Vec<f32> = s
            .split_whitespace()
            .filter_map(|p| p.parse().ok())
            .collect();
        if parts.len() == 3 {
            Some(Vec3::new(parts[0], parts[1], parts[2]))
        } else {
            None
        }
    }
}

/// Loaded BSP map.
#[derive(Debug, Clone, Default)]
pub struct BspMap {
    pub name: String,
    pub version: u32,
    pub map_revision: u32,

    pub entities: Vec<BspEntity>,
    pub planes: Vec<Plane>,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub surf_edges: Vec<i32>,
    pub faces: Vec<Face>,
    pub brushes: Vec<Brush>,
    pub brush_sides: Vec<BrushSide>,
    pub models: Vec<Model>,
}

impl BspMap {
    /// Loads a BSP file from disk.
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
        let mut reader = BufReader::new(file);

        let header = Self::read_header(&mut reader)?;
        let mut map = BspMap {
            name,
            version: header.version,
            map_revision: header.map_revision,
            ..Default::default()
        };

        map.entities = Self::read_entities(&mut reader, &header)?;
        map.planes = Self::read_planes(&mut reader, &header)?;
        map.vertices = Self::read_vertices(&mut reader, &header)?;
        map.edges = Self::read_edges(&mut reader, &header)?;
        map.surf_edges = Self::read_surf_edges(&mut reader, &header)?;
        map.faces = Self::read_faces(&mut reader, &header)?;
        map.brushes = Self::read_brushes(&mut reader, &header)?;
        map.brush_sides = Self::read_brush_sides(&mut reader, &header)?;
        map.models = Self::read_models(&mut reader, &header)?;

        Ok(map)
    }

    fn read_header<R: Read + Seek>(r: &mut R) -> anyhow::Result<BspHeader> {
        let magic = read_u32(r)?;
        if magic != BSP_MAGIC {
            bail!("invalid BSP magic: {:#x}", magic);
        }

        let version = read_u32(r)?;
        if version < BSP_VERSION_MIN || version > BSP_VERSION_MAX {
            bail!("unsupported BSP version: {}", version);
        }

        let mut lumps = [LumpEntry::default(); HEADER_LUMPS];
        for lump in &mut lumps {
            lump.offset = read_u32(r)?;
            lump.length = read_u32(r)?;
            lump.version = read_u32(r)?;
            r.read_exact(&mut lump.fourcc)?;
        }

        let map_revision = read_u32(r)?;

        Ok(BspHeader {
            version,
            lumps,
            map_revision,
        })
    }

    fn read_lump<R: Read + Seek>(
        r: &mut R,
        header: &BspHeader,
        idx: LumpIndex,
    ) -> anyhow::Result<Vec<u8>> {
        let lump = &header.lumps[idx as usize];
        if lump.length == 0 {
            return Ok(Vec::new());
        }
        r.seek(SeekFrom::Start(lump.offset as u64))?;
        let mut data = vec![0u8; lump.length as usize];
        r.read_exact(&mut data)?;
        Ok(data)
    }

    fn read_entities<R: Read + Seek>(
        r: &mut R,
        header: &BspHeader,
    ) -> anyhow::Result<Vec<BspEntity>> {
        let data = Self::read_lump(r, header, LumpIndex::Entities)?;
        let text = String::from_utf8_lossy(&data);
        parse_entity_lump(&text)
    }

    fn read_planes<R: Read + Seek>(r: &mut R, header: &BspHeader) -> anyhow::Result<Vec<Plane>> {
        let data = Self::read_lump(r, header, LumpIndex::Planes)?;
        const SIZE: usize = 20;
        let count = data.len() / SIZE;
        let mut planes = Vec::with_capacity(count);
        for i in 0..count {
            let off = i * SIZE;
            let normal = Vec3::new(
                read_f32_slice(&data[off..])?,
                read_f32_slice(&data[off + 4..])?,
                read_f32_slice(&data[off + 8..])?,
            );
            let dist = read_f32_slice(&data[off + 12..])?;
            let plane_type = read_i32_slice(&data[off + 16..])?;
            planes.push(Plane {
                normal,
                dist,
                plane_type,
            });
        }
        Ok(planes)
    }

    fn read_vertices<R: Read + Seek>(r: &mut R, header: &BspHeader) -> anyhow::Result<Vec<Vertex>> {
        let data = Self::read_lump(r, header, LumpIndex::Vertices)?;
        const SIZE: usize = 12;
        let count = data.len() / SIZE;
        let mut verts = Vec::with_capacity(count);
        for i in 0..count {
            let off = i * SIZE;
            verts.push(Vec3::new(
                read_f32_slice(&data[off..])?,
                read_f32_slice(&data[off + 4..])?,
                read_f32_slice(&data[off + 8..])?,
            ));
        }
        Ok(verts)
    }

    fn read_edges<R: Read + Seek>(r: &mut R, header: &BspHeader) -> anyhow::Result<Vec<Edge>> {
        let data = Self::read_lump(r, header, LumpIndex::Edges)?;
        const SIZE: usize = 4;
        let count = data.len() / SIZE;
        let mut edges = Vec::with_capacity(count);
        for i in 0..count {
            let off = i * SIZE;
            edges.push(Edge {
                v: [
                    read_u16_slice(&data[off..])?,
                    read_u16_slice(&data[off + 2..])?,
                ],
            });
        }
        Ok(edges)
    }

    fn read_surf_edges<R: Read + Seek>(r: &mut R, header: &BspHeader) -> anyhow::Result<Vec<i32>> {
        let data = Self::read_lump(r, header, LumpIndex::SurfEdges)?;
        const SIZE: usize = 4;
        let count = data.len() / SIZE;
        let mut surf_edges = Vec::with_capacity(count);
        for i in 0..count {
            surf_edges.push(read_i32_slice(&data[i * SIZE..])?);
        }
        Ok(surf_edges)
    }

    fn read_faces<R: Read + Seek>(r: &mut R, header: &BspHeader) -> anyhow::Result<Vec<Face>> {
        let data = Self::read_lump(r, header, LumpIndex::Faces)?;
        const SIZE: usize = 56;
        let count = data.len() / SIZE;
        let mut faces = Vec::with_capacity(count);
        for i in 0..count {
            let off = i * SIZE;
            let d = &data[off..];
            faces.push(Face {
                plane_num: read_u16_slice(d)?,
                side: d[2],
                on_node: d[3],
                first_edge: read_i32_slice(&d[4..])?,
                num_edges: read_i16_slice(&d[8..])?,
                tex_info: read_i16_slice(&d[10..])?,
                disp_info: read_i16_slice(&d[12..])?,
                surface_fog_volume_id: read_i16_slice(&d[14..])?,
                styles: [d[16], d[17], d[18], d[19]],
                light_ofs: read_i32_slice(&d[20..])?,
                area: read_f32_slice(&d[24..])?,
                lightmap_mins: [read_i32_slice(&d[28..])?, read_i32_slice(&d[32..])?],
                lightmap_size: [read_i32_slice(&d[36..])?, read_i32_slice(&d[40..])?],
                orig_face: read_i32_slice(&d[44..])?,
                num_prims: read_u16_slice(&d[48..])?,
                first_prim_id: read_u16_slice(&d[50..])?,
                smoothing_groups: read_u32_slice(&d[52..])?,
            });
        }
        Ok(faces)
    }

    fn read_brushes<R: Read + Seek>(r: &mut R, header: &BspHeader) -> anyhow::Result<Vec<Brush>> {
        let data = Self::read_lump(r, header, LumpIndex::Brushes)?;
        const SIZE: usize = 12;
        let count = data.len() / SIZE;
        let mut brushes = Vec::with_capacity(count);
        for i in 0..count {
            let off = i * SIZE;
            brushes.push(Brush {
                first_side: read_i32_slice(&data[off..])?,
                num_sides: read_i32_slice(&data[off + 4..])?,
                contents: read_i32_slice(&data[off + 8..])?,
            });
        }
        Ok(brushes)
    }

    fn read_brush_sides<R: Read + Seek>(
        r: &mut R,
        header: &BspHeader,
    ) -> anyhow::Result<Vec<BrushSide>> {
        let data = Self::read_lump(r, header, LumpIndex::BrushSides)?;
        const SIZE: usize = 8;
        let count = data.len() / SIZE;
        let mut sides = Vec::with_capacity(count);
        for i in 0..count {
            let off = i * SIZE;
            sides.push(BrushSide {
                plane_num: read_u16_slice(&data[off..])?,
                tex_info: read_i16_slice(&data[off + 2..])?,
                disp_info: read_i16_slice(&data[off + 4..])?,
                bevel: read_i16_slice(&data[off + 6..])?,
            });
        }
        Ok(sides)
    }

    fn read_models<R: Read + Seek>(r: &mut R, header: &BspHeader) -> anyhow::Result<Vec<Model>> {
        let data = Self::read_lump(r, header, LumpIndex::Models)?;
        const SIZE: usize = 48;
        let count = data.len() / SIZE;
        let mut models = Vec::with_capacity(count);
        for i in 0..count {
            let off = i * SIZE;
            let d = &data[off..];
            models.push(Model {
                mins: Vec3::new(
                    read_f32_slice(d)?,
                    read_f32_slice(&d[4..])?,
                    read_f32_slice(&d[8..])?,
                ),
                maxs: Vec3::new(
                    read_f32_slice(&d[12..])?,
                    read_f32_slice(&d[16..])?,
                    read_f32_slice(&d[20..])?,
                ),
                origin: Vec3::new(
                    read_f32_slice(&d[24..])?,
                    read_f32_slice(&d[28..])?,
                    read_f32_slice(&d[32..])?,
                ),
                head_node: read_i32_slice(&d[36..])?,
                first_face: read_i32_slice(&d[40..])?,
                num_faces: read_i32_slice(&d[44..])?,
            });
        }
        Ok(models)
    }

    /// Gets spawn points from the entity list.
    pub fn spawn_points(&self) -> Vec<Vec3> {
        self.entities
            .iter()
            .filter(|e| e.classname.starts_with("info_player") || e.classname == "info_target")
            .filter_map(|e| e.origin())
            .collect()
    }

    /// Gets the worldspawn entity.
    pub fn worldspawn(&self) -> Option<&BspEntity> {
        self.entities.iter().find(|e| e.classname == "worldspawn")
    }
}

/// Parses the entity lump text into structured entities.
fn parse_entity_lump(text: &str) -> anyhow::Result<Vec<BspEntity>> {
    let mut entities = Vec::new();
    let mut current: Option<BspEntity> = None;
    let mut in_entity = false;

    for line in text.lines() {
        let line = line.trim();
        if line == "{" {
            in_entity = true;
            current = Some(BspEntity::default());
        } else if line == "}" {
            if let Some(ent) = current.take() {
                entities.push(ent);
            }
            in_entity = false;
        } else if in_entity {
            // Parse "key" "value"
            if let Some((key, value)) = parse_kv_line(line) {
                if let Some(ref mut ent) = current {
                    if key == "classname" {
                        ent.classname = value.to_string();
                    }
                    ent.properties.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    Ok(entities)
}

fn parse_kv_line(line: &str) -> Option<(&str, &str)> {
    // Parse "key" "value" format
    let mut chars = line.char_indices();

    // Find first quote
    let key_start = loop {
        match chars.next() {
            Some((i, '"')) => break i + 1,
            Some(_) => continue,
            None => return None,
        }
    };

    // Find end of key (next quote)
    let key_end = loop {
        match chars.next() {
            Some((i, '"')) => break i,
            Some(_) => continue,
            None => return None,
        }
    };

    // Find start of value (next quote)
    let value_start = loop {
        match chars.next() {
            Some((i, '"')) => break i + 1,
            Some(_) => continue,
            None => return None,
        }
    };

    // Find end of value (next quote)
    let value_end = loop {
        match chars.next() {
            Some((i, '"')) => break i,
            Some(_) => continue,
            None => return None,
        }
    };

    Some((&line[key_start..key_end], &line[value_start..value_end]))
}

// Binary reading helpers.
fn read_u32<R: Read>(r: &mut R) -> anyhow::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u32_slice(d: &[u8]) -> anyhow::Result<u32> {
    Ok(u32::from_le_bytes(d[..4].try_into()?))
}

fn read_i32_slice(d: &[u8]) -> anyhow::Result<i32> {
    Ok(i32::from_le_bytes(d[..4].try_into()?))
}

fn read_u16_slice(d: &[u8]) -> anyhow::Result<u16> {
    Ok(u16::from_le_bytes(d[..2].try_into()?))
}

fn read_i16_slice(d: &[u8]) -> anyhow::Result<i16> {
    Ok(i16::from_le_bytes(d[..2].try_into()?))
}

fn read_f32_slice(d: &[u8]) -> anyhow::Result<f32> {
    Ok(f32::from_le_bytes(d[..4].try_into()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_entity_lump_basic() {
        let text = r#"
{
"classname" "worldspawn"
"mapversion" "1"
}
{
"classname" "info_player_start"
"origin" "0 0 64"
}
"#;
        let ents = parse_entity_lump(text).unwrap();
        assert_eq!(ents.len(), 2);
        assert_eq!(ents[0].classname, "worldspawn");
        assert_eq!(ents[1].classname, "info_player_start");
        assert_eq!(ents[1].origin(), Some(Vec3::new(0.0, 0.0, 64.0)));
    }
}
