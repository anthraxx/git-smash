use std::process::{Command, Stdio};

use crate::errors::*;

pub struct GitConfigBuilder {
    key: &'static str,
    default: Option<&'static str>,
    value_type: Option<&'static str>,
}

impl GitConfigBuilder {
    pub fn new(key: &'static str) -> Self {
        Self {
            key,
            default: None,
            value_type: None,
        }
    }

    pub fn with_default(mut self, default: &'static str) -> Self {
        self.default = Some(default);
        self
    }

    pub fn with_type(mut self, value_type: &'static str) -> Self {
        self.value_type = Some(value_type);
        self
    }

    pub fn get(&self) -> Result<String> {
        let mut args = vec!["config", "--get"];
        if let Some(default) = self.default {
            args.push("--default");
            args.push(default);
        }
        if let Some(value_type) = self.value_type {
            args.push("--type");
            args.push(value_type);
        }
        args.push(self.key);

        let output = Command::new("git")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(&args)
            .output()?;
        if !output.status.success() {
            bail!("{}", String::from_utf8_lossy(output.stderr.as_ref()).trim());
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned())
    }

    pub fn get_as_bool(&mut self) -> Result<bool> {
        let value: bool = self
            .get()?
            .parse()
            .with_context(|| anyhow!("Failed to parse key '{}' as bool", self.key))?;
        Ok(value)
    }
}
