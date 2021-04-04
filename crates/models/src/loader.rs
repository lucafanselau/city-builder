use core::{
    anyhow::{bail, Context, Result},
    log,
};
use std::sync::Arc;

use app::{AssetHandle, AssetLoader, LoadContext};
use artisan::{
    material::{Material, SolidMaterial},
    mesh::{Indices, Mesh, MeshPart, Model, Vertex},
    prelude::glam,
    renderer::ActiveContext,
};
use gltf::{mesh::util::ReadIndices, Gltf, Semantic};
use render::{
    context::GpuContext,
    prelude::{BufferUsage, MemoryType},
    resource::buffer::BufferDescriptor,
};
use tasks::futures;

// fn load_node(
//     node: gltf::Node,
//     vertices: &mut Vec<Vertex>,
//     indices: &mut Vec<Index>,
//     buffers: &[Vec<u8>],
//     transform: Option<glam::Mat4>,
// ) {
//     let mut local_transform = transform.unwrap_or_else(glam::Mat4::identity);
//     local_transform = local_transform * glam::Mat4::from_cols_array_2d(&node.transform().matrix());

//     if let Some(mesh) = node.mesh() {
//         log::debug!("[GltfLoader] loading mesh {:?}", mesh.name());
//         for primitive in mesh.primitives() {
//             if let Some(accessor) = primitive.get(&Semantic::Positions) {
//                 log::info!("Found Positions accessor: {}", accessor.size());
//             }
//         }
//     }

//     for child in node.children() {
//         load_node(child, vertices, indices, buffers, Some(local_transform))
//     }
// }
//

async fn load_meshes<'a>(
    doc: &'a Gltf,
    ctx: &'a ActiveContext,
    buffers: &'a [Vec<u8>],
    load_ctx: &LoadContext<'a>,
) -> Result<Vec<AssetHandle<Mesh>>> {
    tasks::utilities::try_join_all(doc.meshes().map(|mesh| async move {
        let mesh_name = mesh
            .name()
            .context("[GltfLoader] mesh must always have a name")?;

        let parts = mesh
            .primitives()
            .enumerate()
            .map(|(i, p)| {
                let reader = p.reader(|b| buffers.get(b.index()).map(|d| d.as_slice()));
                let positions = reader
                    .read_positions()
                    .context("[GltfLoader] meshes must always have position attributes")?;
                let normals = reader
                    .read_normals()
                    .context("[GltfLoader] meshes must always have normal attributes")?;

                let vertices: Vec<Vertex> = positions
                    .zip(normals)
                    .map(|(pos, normal)| Vertex {
                        pos: pos.into(),
                        normal: normal.into(),
                    })
                    .collect();

                let gltf_indices = reader
                    .read_indices()
                    .context("[GltfLoader] meshes must always have indices")?;

                let indices = match gltf_indices {
                    ReadIndices::U8(_) => {
                        bail!("[GltfLoader] u8 indices are unsupported!")
                    }
                    ReadIndices::U16(data) => Indices::U16(data.collect::<Vec<u16>>()),
                    ReadIndices::U32(data) => Indices::U32(data.collect::<Vec<u32>>()),
                };

                let material = {
                    let pbr = p.material().pbr_metallic_roughness();

                    let color = pbr.base_color_factor().into();

                    Material::solid(color * 0.2, color, color, 0.1)
                };

                let part = MeshPart::from_data(
                    &format!("{}-primitive-{}", mesh_name, i),
                    &vertices,
                    &indices,
                    material,
                    ctx,
                );

                Ok(part)
            })
            .collect::<Result<Vec<_>>>()?;

        let handle = load_ctx
            .add_asset_with_label(mesh_name, Mesh::new(mesh_name, parts))
            .await;

        Ok(handle)
    }))
    .await
}

const URI_STRING: &str = "data:application/octet-stream;base64,";

fn load_buffers(doc: &Gltf) -> core::anyhow::Result<Vec<Vec<u8>>> {
    let mut result = Vec::new();
    for buffer in doc.buffers() {
        use gltf::buffer::Source::*;
        match buffer.source() {
            Bin => {
                let data = doc.blob.clone().context("failed to read binary data")?;
                result.push(data.clone());
            }
            Uri(uri) => {
                if uri.starts_with("data:") {
                    if let Some(stripped) = uri.strip_prefix(URI_STRING) {
                        let data = base64::decode(stripped)?;
                        result.push(data);
                    } else {
                        bail!(
                            "Gltf data uri has unkown format: {:?}",
                            uri.split_once(',').unwrap().0
                        )
                    }
                } else {
                    bail!("Actual uri's are not supported")
                }
            }
        }
    }
    Ok(result)
}

fn load_model(doc: &Gltf, meshes: &[AssetHandle<Mesh>]) -> Result<Model> {
    let default_scene = doc
        .default_scene()
        .context("[GltfLoader] (load_model) default scene is required")?;

    fn load_node(
        model: &mut Model,
        node: gltf::Node,
        meshes: &[AssetHandle<Mesh>],
        transform: Option<glam::Mat4>,
    ) {
        let node_transform = glam::Mat4::from_cols_array_2d(&node.transform().matrix());
        let transform = transform.unwrap_or_else(glam::Mat4::identity) * node_transform;

        if let Some(mesh) = node.mesh() {
            let mesh_handle = meshes[mesh.index()].clone();
            model.add_mesh(transform, mesh_handle);
        }

        for child in node.children() {
            load_node(model, child, meshes, Some(transform));
        }
    }

    let mut model = Model::new();
    for node in default_scene.nodes() {
        load_node(&mut model, node, meshes, None);
    }

    Ok(model)
}

pub struct GltfLoader {
    pub(crate) ctx: Arc<ActiveContext>,
}

impl AssetLoader for GltfLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        ctx: LoadContext<'a>,
    ) -> app::BoxedFuture<'a, core::anyhow::Result<()>> {
        Box::pin(async move {
            let model: Gltf = gltf::Gltf::from_slice(bytes)?;

            let buffers = load_buffers(&model).context("[GltfLoader] load_buffers failed")?;
            let meshes = load_meshes(&model, &self.ctx, &buffers, &ctx)
                .await
                .context("[GltfLoader] load_meshes failed")?;
            let model = load_model(&model, &meshes)?;
            ctx.send_asset(model).await;

            Ok(())
        })
    }

    fn ext(&self) -> &[&str] {
        &["gltf"]
    }
}
