use std::sync::Arc;

use app::core::anyhow::{anyhow, Context, Result};
use app::{App, AssetLoader};
use render::prelude::{GpuContext, ShaderType};

use crate::renderer::ActiveContext;

#[derive(Debug)]
pub struct ShaderAsset(pub Vec<u32>);

pub struct ShaderLoader {
    ctx: Arc<ActiveContext>,
}

// Small utility to convert Osstr to str
fn convert_os_str(input: Option<&std::ffi::OsStr>) -> Result<&str> {
    let os_str = input.ok_or_else(|| anyhow!("failed to parse os str"))?;
    os_str
        .to_str()
        .ok_or_else(|| anyhow!("failed to convert OsStr to str: {:?}", os_str))
}

impl AssetLoader for ShaderLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        ctx: app::LoadContext<'a>,
    ) -> app::BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            let content = std::str::from_utf8(bytes).context(format!(
                "[ShaderLoader] failed to parse input as utf8 str: {:?}",
                ctx.path
            ))?;
            // now we need to figure out the type
            let extension = convert_os_str(ctx.path.extension())?;
            let name = convert_os_str(ctx.path.file_name())?;

            let shader_type = match extension {
                "vert" => ShaderType::Vertex,
                "frag" => ShaderType::Fragment,
                _ => panic!(),
            };
            let result = self
                .ctx
                .compile_shader(render::prelude::ShaderSource::GlslSource {
                    source: content,
                    shader_type,
                    name: Some(name),
                });
            // And then send the asset
            ctx.server
                .channels
                .get_channel::<ShaderAsset>()
                .send(ctx.handle.typed(), ShaderAsset(result))
                .await;

            Ok(())
        })
    }

    fn ext(&self) -> &[&str] {
        &["vert", "frag"]
    }
}

pub(crate) fn init(app: &mut App, ctx: Arc<ActiveContext>) {
    app.register_asset::<ShaderAsset>();
    app.add_asset_loader(ShaderLoader { ctx });
}
