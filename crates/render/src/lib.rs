pub mod context;
pub mod gfx;
pub mod resource;

/// This is where we want to go (at least) roughly with our implementation
// struct Graph {}
// impl Graph {
//     fn new() -> Self {
//         Self {}
//     }
//     fn add_node<T: Node>(&mut self, node: T) -> T::Output {
//         0
//     }
// }
//
// trait Node {
//     type Output;
// }
//
// struct DynamicAssetNode {}
// impl DynamicAssetNode {
//     fn new(handle: BufferHandle) -> Self {
//         DynamicAssetNode {}
//     }
// }
//
// impl Node for DynamicAssetNode {
//     type Output = u64;
// }
//
// struct PassNode {}
// impl PassNode {}
//
// struct BufferHandle(u64);

fn vision() {
    // Suppose we have a simple rendering with a shadow system
    // We would need to have a shadow_pass that has a specific camera,
    //  -> The camera from the sun's or light source position
    // and a texture output (which is essentially a depth buffer)
    // it is feeded, just the geometry of the scene

    // Followed by a main pass that takes this depth buffer, and probably a lot of material information,
    // some additional light sources etc. and create the final image, which is linked to the backbuffer
    // let mut frame_graph = Graph::new();
    //
    // let buffer = BufferHandle(0u64); // more like: context.create_initialized_buffer<T: IntoRenderResource>(t: T);
    //
    // // This should trigger an update of the Internal Buffer, every time we update the initial buffer
    // // like context.update_buffer<T: IntoRenderResource>(buffer, new: T)
    // let sun_camera = frame_graph.add_node(DynamicAssetNode::new(
    //     ""
    //     buffer /*Of maybe like a type or buffer (with PhantomMarker)*/
    // ));
    //
    // let depth_attachment = frame_graph.create_depth_attachment(GraphAttachment::DepthAttachment {
    //     name: "Depth Attachment".into(),
    //     format: DepthFormat::Default,
    //     size: Size::Percent(100.0, 100.0),
    // });
    //
    // let materials: Vec<u64> = vec![];
    //
    // let materials_node = frame_graph.add_node(DynamicAssetNode::new("Materials", 0u64));
    //
    // let shadow_pass = frame_graph.add_node(
    //     PassNodeBuilder::new()
    //         .set_name("SHADOW_PASS")
    //         .add_depth_attachment(depth_attachment)
    //         .add_node_dependencies([ sun_camera.clone() ])
    //         .add_callback(|world, resources, render_commands| {
    //             // And then something in the lines of
    //             // NOTE(luca): framebuffer setup is reflected in outer RenderPass, other things are passed over by descriptor sets and push_constants?
    //             render_commands.set_pipeline(/* my graphics pipeline -> should be compatible with what is mentioned above (do we need to assert this?)*/);
    //             // This seems kinda nice for the values
    //             // render_commands.set_descriptor_sets(descriptor_sets);
    //             for (_e, (mesh_component, transform)) in world.query::<(&MeshComponent, &Transform)>() {
    //                 // Transform needs to be provided over push_constants (since uniform buffer would be horrible?)
    //                 render_commands.push_constants(Transform::to_push_constant(transform));
    //                 render_commands.bind_vertex_buffer(mesh_component.buffer);
    //                 render_commands.draw(mesh_component.count);
    //             }
    //         })
    //         .build(),
    // );
    //
    // let my_fancy_pbr_pipeline = 0u64; // more like PipelineServer:
    //
    // let main_pass = frame_graph.add_node(
    //     PassNodeBuilder::new()
    //         .set_name("MAIN_PASS")
    //         .add_sampled_texture(depth_attachment)
    //         .add_node_dependencies([shadow_pass.clone(), materials_node.clone()])
    //         .add_color_attachment(frame_graph.get_backbuffer())
    //         .add_callback(|world, resources, render_commands| {
    //             render_commands.set_pipeline(my_fancy_pbr_pipeline);
    //             render_commands.set_descriptor(some_top_notch_abstraction_for_that);
    //             for (_e, (mesh_component, transform)) in
    //                 world.query::<(&MeshComponent, &Transform)>()
    //             {
    //                 render_commands.set_vertex_buffe
    //             }
    //         })
    //         .build(),
    // );
    //
    // frame_graph.bake();
}
