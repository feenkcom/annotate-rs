use crate::{AnnotatedFunction, AnnotatedModule};

pub struct Environment {
    pub(crate) modules: Vec<AnnotatedModule>,
    pub(crate) functions: Vec<AnnotatedFunction>,
}

impl Environment {
    pub fn export(&self) {
        crate::generate(
            crate::tokenize(self.modules.as_slice(), self.functions.as_slice()),
            "annotate.rs",
        );
    }
}
