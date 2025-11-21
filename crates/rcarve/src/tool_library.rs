use crate::types::Tool;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a persisted collection of tools stored on disk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolLibrary {
    pub tools: Vec<Tool>,
}

impl ToolLibrary {
    /// Create an empty library.
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Load a library from the provided path. Missing files yield an empty library.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Self::new());
        }

        let data =
            fs::read(path).with_context(|| format!("read tool library {}", path.display()))?;
        let library: ToolLibrary =
            serde_json::from_slice(&data).context("deserialize tool library")?;
        Ok(library)
    }

    /// Persist the library to the provided path, ensuring the directory exists.
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create tool library directory {}", parent.display()))?;
        }

        let data =
            serde_json::to_vec_pretty(self).context("serialize tool library to JSON bytes")?;
        fs::write(path, data).with_context(|| format!("write tool library {}", path.display()))
    }

    /// Append a new tool to the library.
    pub fn add_tool(&mut self, tool: Tool) {
        self.tools.push(tool);
    }

    /// Update an existing tool at the provided index.
    pub fn update_tool(&mut self, index: usize, tool: Tool) -> Result<()> {
        let slot = self
            .tools
            .get_mut(index)
            .ok_or_else(|| anyhow!("invalid tool index {index}"))?;
        *slot = tool;
        Ok(())
    }

    /// Remove a tool at the provided index.
    pub fn remove_tool(&mut self, index: usize) -> Result<()> {
        if index >= self.tools.len() {
            return Err(anyhow!("invalid tool index {index}"));
        }
        self.tools.remove(index);
        Ok(())
    }

    /// Resolve the default library path (`~/.rcarve/tools/library.json`), creating directories.
    pub fn default_library_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("could not determine home directory"))?;
        let path = home.join(".rcarve").join("tools").join("library.json");

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create tool library directory {}", parent.display()))?;
        }

        Ok(path)
    }
}
