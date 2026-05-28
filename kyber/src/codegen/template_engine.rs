use tera::{Tera, Context};
use anyhow::Result;
use include_dir::{Dir, include_dir};

static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates");

pub struct TemplateEngine {
    tera: Tera,
}

impl TemplateEngine {
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();
        load_templates_recursive(&TEMPLATES_DIR, &mut tera)?;
        Ok(TemplateEngine { tera })
    }

    pub fn render(&self, template_name: &str, context: &Context) -> Result<String> {
        let rendered = self.tera.render(template_name, context)?;
        Ok(rendered)
    }
}

fn load_templates_recursive(dir: &Dir, tera: &mut Tera) -> Result<()> {
    for file in dir.files() {
        let path = file.path();
        if let Some(ext) = path.extension() {
            if ext == "tera" {
                let relative = path.strip_prefix("/").unwrap_or(path);
                let name = relative.to_string_lossy().to_string();
                let content = std::str::from_utf8(file.contents())?;
                tera.add_raw_template(&name, content)?;
            }
        }
    }
    for subdir in dir.dirs() {
        load_templates_recursive(subdir, tera)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_templates_load_without_error() {
        let engine = TemplateEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_base_templates_render() {
        let engine = TemplateEngine::new().unwrap();
        let mut ctx = Context::new();
        ctx.insert("agent_name", "test-agent");
        ctx.insert("has_terminal", &true);
        ctx.insert("has_filesystem", &true);
        ctx.insert("has_browser", &false);
        ctx.insert("has_desktop", &false);
        ctx.insert("has_network", &false);
        ctx.insert("has_git", &false);
        let result = engine.render("base/Cargo.toml.tera", &ctx);
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("test-agent"));
    }
}
