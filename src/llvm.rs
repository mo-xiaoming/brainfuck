use llvm_sys::core;
use llvm_sys::prelude;

pub struct Env {}

impl Env {
    pub fn new() -> Self {
        Self {}
    }

    pub fn create_context(&self) -> ContextRef {
        ContextRef::new()
    }
}
/*
impl Drop for Env {
    fn drop(&mut self) {
        unsafe {
            core::LLVMShutdown();
        }
    }
}
*/
pub struct ContextRef(prelude::LLVMContextRef);

impl Drop for ContextRef {
    fn drop(&mut self) {
        unsafe {
            core::LLVMContextDispose(self.0);
        }
    }
}

impl ContextRef {
    fn new() -> Self {
        Self(unsafe { core::LLVMContextCreate() })
    }

    pub fn create_module_with_name(&self, name: &str) -> ModuleRef {
        ModuleRef::new(name, self)
    }

    pub fn create_builder(&self) -> BuilderRef {
        BuilderRef::new(self)
    }
}

pub struct ModuleRef(prelude::LLVMModuleRef);

impl ModuleRef {
    fn new(name: &str, ctx: &ContextRef) -> Self {
        Self(unsafe { core::LLVMModuleCreateWithNameInContext(name.as_ptr() as _, ctx.0) })
    }
}
/*
impl Drop for ModuleRef {
    fn drop(&mut self) {
        unsafe {
            core::LLVMDisposeModule(self.0);
        }
    }
}
*/
pub struct BuilderRef(pub prelude::LLVMBuilderRef);

impl BuilderRef {
    fn new(ctx: &ContextRef) -> Self {
        Self(unsafe { core::LLVMCreateBuilderInContext(ctx.0) })
    }
}

impl Drop for BuilderRef {
    fn drop(&mut self) {
        unsafe {
            core::LLVMDisposeBuilder(self.0);
        }
    }
}
