//! Console system.
//!
//! Provides:
//! - Console variables (cvars) with typed values
//! - Command registration and execution
//! - Command history
//! - Input parsing
//!
//! # Usage
//! ```ignore
//! let mut console = Console::new();
//! console.register_cvar("sv_cheats", CvarValue::Int(0));
//! console.register_command("map", |args, ctx| { /* load map */ Ok(()) });
//! console.exec("map de_dust2")?;
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::{bail, Context};

/// Console variable value.
#[derive(Debug, Clone, PartialEq)]
pub enum CvarValue {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

impl CvarValue {
    pub fn as_int(&self) -> Option<i64> {
        match self {
            CvarValue::Int(v) => Some(*v),
            CvarValue::Float(v) => Some(*v as i64),
            CvarValue::Bool(v) => Some(if *v { 1 } else { 0 }),
            CvarValue::String(s) => s.parse().ok(),
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            CvarValue::Float(v) => Some(*v),
            CvarValue::Int(v) => Some(*v as f64),
            CvarValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            CvarValue::Bool(v) => *v,
            CvarValue::Int(v) => *v != 0,
            CvarValue::Float(v) => *v != 0.0,
            CvarValue::String(s) => !s.is_empty() && s != "0" && s.to_lowercase() != "false",
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            CvarValue::String(s) => s.clone(),
            CvarValue::Int(v) => v.to_string(),
            CvarValue::Float(v) => v.to_string(),
            CvarValue::Bool(v) => v.to_string(),
        }
    }
}

impl std::fmt::Display for CvarValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CvarValue::Int(v) => write!(f, "{}", v),
            CvarValue::Float(v) => write!(f, "{}", v),
            CvarValue::String(v) => write!(f, "\"{}\"", v),
            CvarValue::Bool(v) => write!(f, "{}", v),
        }
    }
}

/// Console variable metadata.
#[derive(Debug, Clone)]
pub struct Cvar {
    pub name: String,
    pub value: CvarValue,
    pub default: CvarValue,
    pub description: String,
    pub flags: CvarFlags,
}

bitflags::bitflags! {
    /// Cvar flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CvarFlags: u32 {
        const NONE = 0;
        const ARCHIVE = 1 << 0;      // Saved to config
        const CHEAT = 1 << 1;        // Requires sv_cheats
        const REPLICATED = 1 << 2;   // Server -> client
        const SERVER_ONLY = 1 << 3;  // Server-side only
    }
}

impl Default for CvarFlags {
    fn default() -> Self {
        Self::NONE
    }
}

/// Command handler function type.
pub type CommandHandler = Box<dyn Fn(&[&str], &mut ConsoleContext) -> anyhow::Result<()> + Send + Sync>;

/// Context passed to command handlers.
pub struct ConsoleContext {
    /// Output buffer for command responses.
    pub output: Vec<String>,
    /// Reference to cvars (for commands that need to read/write them).
    pub cvars: Arc<RwLock<HashMap<String, Cvar>>>,
}

impl ConsoleContext {
    pub fn print(&mut self, msg: impl Into<String>) {
        self.output.push(msg.into());
    }

    pub fn get_cvar(&self, name: &str) -> Option<CvarValue> {
        self.cvars.read().ok()?.get(name).map(|c| c.value.clone())
    }

    pub fn set_cvar(&self, name: &str, value: CvarValue) -> anyhow::Result<()> {
        let mut cvars = self.cvars.write().map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        if let Some(cvar) = cvars.get_mut(name) {
            cvar.value = value;
            Ok(())
        } else {
            bail!("unknown cvar: {}", name);
        }
    }
}

/// The console.
pub struct Console {
    cvars: Arc<RwLock<HashMap<String, Cvar>>>,
    commands: HashMap<String, CommandHandler>,
    history: Vec<String>,
    max_history: usize,
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

impl Console {
    pub fn new() -> Self {
        let mut console = Self {
            cvars: Arc::new(RwLock::new(HashMap::new())),
            commands: HashMap::new(),
            history: Vec::new(),
            max_history: 100,
        };

        // Register built-in commands.
        console.register_builtin_commands();
        console
    }

    fn register_builtin_commands(&mut self) {
        // echo <text>
        self.register_command("echo", |args, ctx| {
            ctx.print(args.join(" "));
            Ok(())
        });

        // help [command]
        self.register_command("help", |args, ctx| {
            if args.is_empty() {
                ctx.print("Available commands: echo, help, cvarlist, set, quit");
            } else {
                ctx.print(format!("Help for '{}': not implemented", args[0]));
            }
            Ok(())
        });

        // cvarlist
        self.register_command("cvarlist", |_args, ctx| {
            let cvars = ctx.cvars.read().map_err(|_| anyhow::anyhow!("lock"))?;
            let lines: Vec<String> = cvars
                .iter()
                .map(|(name, cvar)| format!("  {} = {} (default: {})", name, cvar.value, cvar.default))
                .collect();
            drop(cvars);
            for line in lines {
                ctx.print(line);
            }
            Ok(())
        });

        // set <cvar> <value>
        self.register_command("set", |args, ctx| {
            if args.len() < 2 {
                bail!("usage: set <cvar> <value>");
            }
            let name = args[0];
            let value_str = args[1..].join(" ");

            // Try to parse as int, then float, then string.
            let value = if let Ok(v) = value_str.parse::<i64>() {
                CvarValue::Int(v)
            } else if let Ok(v) = value_str.parse::<f64>() {
                CvarValue::Float(v)
            } else if value_str == "true" {
                CvarValue::Bool(true)
            } else if value_str == "false" {
                CvarValue::Bool(false)
            } else {
                CvarValue::String(value_str.trim_matches('"').to_string())
            };

            let value_clone = value.clone();
            ctx.set_cvar(name, value)?;
            ctx.print(format!("{} = {}", name, value_clone));
            Ok(())
        });
    }

    /// Registers a console variable.
    pub fn register_cvar(&mut self, name: &str, default: CvarValue, description: &str, flags: CvarFlags) {
        let cvar = Cvar {
            name: name.to_string(),
            value: default.clone(),
            default,
            description: description.to_string(),
            flags,
        };
        self.cvars.write().unwrap().insert(name.to_string(), cvar);
    }

    /// Registers a command.
    pub fn register_command<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&[&str], &mut ConsoleContext) -> anyhow::Result<()> + Send + Sync + 'static,
    {
        self.commands.insert(name.to_string(), Box::new(handler));
    }

    /// Executes a console command line.
    pub fn exec(&mut self, line: &str) -> anyhow::Result<Vec<String>> {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            return Ok(Vec::new());
        }

        // Add to history.
        self.history.push(line.to_string());
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        // Parse command and arguments.
        let tokens = parse_command_line(line);
        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        let cmd_name = &tokens[0];
        let args: Vec<&str> = tokens[1..].iter().map(|s| s.as_str()).collect();

        let mut ctx = ConsoleContext {
            output: Vec::new(),
            cvars: Arc::clone(&self.cvars),
        };

        // Check if it's a cvar query/set (just typing the name).
        if self.commands.get(cmd_name.as_str()).is_none() {
            let cvar_info = self.cvars.read().ok().and_then(|cvars| {
                cvars.get(cmd_name.as_str()).map(|cvar| {
                    (cvar.name.clone(), cvar.value.clone(), cvar.default.clone())
                })
            });

            if let Some((name, value, default)) = cvar_info {
                if args.is_empty() {
                    ctx.print(format!("{} = {} (default: {})", name, value, default));
                    return Ok(ctx.output);
                } else {
                    // Set cvar.
                    return self.exec(&format!("set {} {}", cmd_name, args.join(" ")));
                }
            }
        }

        // Execute command.
        if let Some(handler) = self.commands.get(cmd_name.as_str()) {
            handler(&args, &mut ctx).with_context(|| format!("command '{}'", cmd_name))?;
        } else {
            ctx.print(format!("Unknown command: {}", cmd_name));
        }

        Ok(ctx.output)
    }

    /// Gets a cvar value.
    pub fn get_cvar(&self, name: &str) -> Option<CvarValue> {
        self.cvars.read().ok()?.get(name).map(|c| c.value.clone())
    }

    /// Sets a cvar value.
    pub fn set_cvar(&self, name: &str, value: CvarValue) -> anyhow::Result<()> {
        let mut cvars = self.cvars.write().map_err(|_| anyhow::anyhow!("lock"))?;
        if let Some(cvar) = cvars.get_mut(name) {
            cvar.value = value;
            Ok(())
        } else {
            bail!("unknown cvar: {}", name);
        }
    }

    /// Gets command history.
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// Gets a shared reference to cvars for use in handlers.
    pub fn cvars_ref(&self) -> Arc<RwLock<HashMap<String, Cvar>>> {
        Arc::clone(&self.cvars)
    }
}

/// Parses a command line into tokens, respecting quotes.
fn parse_command_line(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_cvar_roundtrip() {
        let mut console = Console::new();
        console.register_cvar("test_var", CvarValue::Int(42), "Test variable", CvarFlags::NONE);

        assert_eq!(console.get_cvar("test_var"), Some(CvarValue::Int(42)));

        console.exec("set test_var 100").unwrap();
        assert_eq!(console.get_cvar("test_var"), Some(CvarValue::Int(100)));
    }

    #[test]
    fn parse_quoted_args() {
        let tokens = parse_command_line(r#"say "hello world" test"#);
        assert_eq!(tokens, vec!["say", "hello world", "test"]);
    }
}
