macro_rules! ordered_passes_renderpass_korangar {
    (
        $device:expr,
        attachments: {
            $(
                $atch_name:ident: {
                    load: $load:ident,
                    store: $store:ident,
                    format: $format:expr,
                    samples: $samples:expr,
                    $(initial_layout: $init_layout:expr,)*
                    $(final_layout: $final_layout:expr,)*
                }
            ),*
        },
        passes: [
            $(
                {
                    color: [$($color_atch:ident),*],
                    depth_stencil: {$($depth_atch:ident)*},
                    input: [$($input_atch:ident),*]$(,)*
                    $(resolve: [$($resolve_atch:ident),*])*$(,)*
                }
            ),*
        ]
    ) => ({
        use vulkano::render_pass::RenderPass;

        let desc = {
            use vulkano::render_pass::AttachmentDesc;
            use vulkano::render_pass::RenderPassDesc;
            use vulkano::render_pass::SubpassDependencyDesc;
            use vulkano::render_pass::SubpassDesc;
            use vulkano::image::ImageLayout;
            use vulkano::sync::AccessFlags;
            use vulkano::sync::PipelineStages;
            use std::convert::TryInto;

            let mut attachment_num = 0;
            $(
                let $atch_name = attachment_num;
                attachment_num += 1;
            )*

            let mut layouts: Vec<(Option<ImageLayout>, Option<ImageLayout>)> = vec![(None, None); attachment_num];

            let subpasses = vec![
                $({
                    let desc = SubpassDesc {
                        color_attachments: vec![
                            $({
                                let layout = &mut layouts[$color_atch];
                                layout.0 = layout.0.or(Some(ImageLayout::ColorAttachmentOptimal));
                                layout.1 = Some(ImageLayout::ColorAttachmentOptimal);

                                ($color_atch, ImageLayout::ColorAttachmentOptimal)
                            }),*
                        ],
                        depth_stencil: {
                            let depth: Option<(usize, ImageLayout)> = None;
                            $(
                                let layout = &mut layouts[$depth_atch];
                                layout.1 = Some(ImageLayout::DepthStencilAttachmentOptimal);
                                layout.0 = layout.0.or(layout.1);

                                let depth = Some(($depth_atch, ImageLayout::DepthStencilAttachmentOptimal));
                            )*
                            depth
                        },
                        input_attachments: vec![
                            $({
                                let layout = &mut layouts[$input_atch];
                                layout.1 = Some(ImageLayout::ShaderReadOnlyOptimal);
                                layout.0 = layout.0.or(layout.1);

                                ($input_atch, ImageLayout::ShaderReadOnlyOptimal)
                            }),*
                        ],
                        resolve_attachments: vec![
                            $($({
                                let layout = &mut layouts[$resolve_atch];
                                layout.1 = Some(ImageLayout::TransferDstOptimal);
                                layout.0 = layout.0.or(layout.1);

                                ($resolve_atch, ImageLayout::TransferDstOptimal)
                            }),*)*
                        ],
                        preserve_attachments: (0 .. attachment_num).filter(|&a| {
                            $(if a == $color_atch { return false; })*
                            $(if a == $depth_atch { return false; })*
                            $(if a == $input_atch { return false; })*
                            $($(if a == $resolve_atch { return false; })*)*
                            true
                        }).collect()
                    };

                    assert!(desc.resolve_attachments.is_empty() ||
                            desc.resolve_attachments.len() == desc.color_attachments.len());
                    desc
                }),*
            ];

            let dependencies = vec![

                /*SubpassDependencyDesc {
                    source_subpass: !0,
                    destination_subpass: 0,
                    source_stages: PipelineStages {
                        bottom_of_pipe: true,
                        ..PipelineStages::none()
                    },
                    destination_stages: PipelineStages {
                        color_attachment_output: true,
                        ..PipelineStages::none()
                    },
                    source_access: AccessFlags{
                       memory_read: true,
                       ..AccessFlags::none()
                    },
                    destination_access: AccessFlags{
                        color_attachment_read: true,
                        color_attachment_write: true,
                        ..AccessFlags::none()
                    },
                    by_region: true,
                },*/

                SubpassDependencyDesc {
                    source_subpass: 0,
                    destination_subpass: 1,
                    source_stages: PipelineStages {
                        color_attachment_output: true,
                        late_fragment_tests: true,
                        ..PipelineStages::none()
                    },
                    destination_stages: PipelineStages {
                        fragment_shader: true,
                        //vertex_input: true,
                        ..PipelineStages::none()
                    },
                    source_access: AccessFlags {
                        color_attachment_write: true,
                        depth_stencil_attachment_write: true,
                        ..AccessFlags::none()
                    },
                    destination_access: AccessFlags{
                        input_attachment_read: true,
                        shader_read: true,
                        //vertex_attribute_read: true,
                        ..AccessFlags::none()
                    },
                    by_region: true,
                }/*,

                SubpassDependencyDesc {
                    source_subpass: 0,
                    destination_subpass: 1,
                    source_stages: PipelineStages {
                        color_attachment_output: true,
                        ..PipelineStages::none()
                    },
                    destination_stages: PipelineStages {
                        fragment_shader: true,
                        ..PipelineStages::none()
                    },
                    source_access: AccessFlags{
                        color_attachment_write: true,
                        depth_stencil_attachment_read: true,
                        depth_stencil_attachment_write: true,
                        ..AccessFlags::none()
                    },
                    destination_access: AccessFlags{
                        shader_read: true,
                        //color_attachment_write: true,
                        ..AccessFlags::none()
                    },
                    by_region: true,
                }*/
            ];

            /*let dependencies = (0..subpasses.len().saturating_sub(1))
                .map(|id| {
                    SubpassDependencyDesc {
                        source_subpass: id,
                        destination_subpass: id + 1,
                        source_stages: PipelineStages {
                            color_attachment_output: true,
                            //all_graphics: true,
                            ..PipelineStages::none()
                        },
                        destination_stages: PipelineStages {
                            fragment_shader: true,
                            //all_graphics: true,
                            ..PipelineStages::none()
                        },
                        source_access: AccessFlags{
                           color_attachment_write: true,
                           ..AccessFlags::none()
                        },
                        destination_access: AccessFlags{
                            shader_read: true,
                            //color_attachment_write: true,
                            ..AccessFlags::none()
                        },
                        by_region: true,
                    }
                })
                .collect();*/

            let attachments = vec![
                $({
                    let layout = &mut layouts[$atch_name];
                    $(layout.0 = Some($init_layout);)*
                    $(layout.1 = Some($final_layout);)*

                    AttachmentDesc {
                        format: $format,
                        samples: $samples.try_into().unwrap(),
                        load: vulkano::render_pass::LoadOp::$load,
                        store: vulkano::render_pass::StoreOp::$store,
                        stencil_load: vulkano::render_pass::LoadOp::$load,
                        stencil_store: vulkano::render_pass::StoreOp::$store,
                        initial_layout: layout.0.expect(
                            format!(
                                "Attachment {} is missing initial_layout, this is normally \
                                automatically determined but you can manually specify it for an individual \
                                attachment in the single_pass_renderpass! macro",
                                attachment_num
                            )
                            .as_ref(),
                        ),
                        final_layout: layout.1.expect(
                            format!(
                                "Attachment {} is missing final_layout, this is normally \
                                automatically determined but you can manually specify it for an individual \
                                attachment in the single_pass_renderpass! macro",
                                attachment_num
                            )
                            .as_ref(),
                        ),
                    }
                }),*
            ];

            RenderPassDesc::new(
                attachments,
                subpasses,
                dependencies,
            )
        };

        RenderPass::new($device, desc)
    });
}
