use wgpu::wgt::{Dx12SwapchainKind, Dx12UseFrameLatencyWaitableObject, WgpuHasDisplayHandle};
use wgpu::{
    BackendOptions, Backends, Dx12BackendOptions, Dx12Compiler, ForceShaderModelToken, GlBackendOptions, GlDebugFns, GlFenceBehavior,
    Gles3MinorVersion, InstanceDescriptor, InstanceFlags, MemoryBudgetThresholds, NoopBackendOptions,
};

/// Connect GLES to the actual window display rather than an implicit
/// surfaceless EGL display. The owned handle keeps that connection alive for
/// the instance.
pub(crate) fn window_instance_descriptor(display: impl WgpuHasDisplayHandle) -> InstanceDescriptor {
    InstanceDescriptor {
        backends: Backends::all().with_env(),
        flags: InstanceFlags::from_build_config().with_env(),
        memory_budget_thresholds: MemoryBudgetThresholds::default(),
        backend_options: BackendOptions {
            gl: GlBackendOptions {
                gles_minor_version: Gles3MinorVersion::Automatic,
                fence_behavior: GlFenceBehavior::Normal,
                debug_fns: GlDebugFns::Auto,
            },
            dx12: Dx12BackendOptions {
                shader_compiler: Dx12Compiler::StaticDxc.with_env(),
                presentation_system: Dx12SwapchainKind::DxgiFromHwnd,
                latency_waitable_object: Dx12UseFrameLatencyWaitableObject::Wait,
                force_shader_model: ForceShaderModelToken::default(),
                agility_sdk: None,
            },
            noop: NoopBackendOptions { enable: false },
        },
        // On Vulkan, Metal and Dx12, this is currently unused.
        display: Some(Box::new(display)),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use winit::raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle};

    use super::*;

    #[derive(Debug)]
    struct TestDisplay(Arc<AtomicBool>);

    impl HasDisplayHandle for TestDisplay {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            Err(HandleError::Unavailable)
        }
    }

    impl Drop for TestDisplay {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn window_descriptor_owns_its_required_display_connection() {
        // No window, graphics instance, device or rendering is involved.
        let dropped = Arc::new(AtomicBool::new(false));
        let descriptor = window_instance_descriptor(TestDisplay(Arc::clone(&dropped)));
        assert!(descriptor.display.is_some());
        assert!(!dropped.load(Ordering::SeqCst));
        drop(descriptor);
        assert!(dropped.load(Ordering::SeqCst));
    }
}
