use core::any::TypeId;
use core::ops::Index;
use core::slice::Iter;
use crate::internal::const_str_equal;

#[derive(Debug, Clone)]
pub struct Attribute {
    pub(crate) name: &'static str,
    pub(crate) value: Value,
}

impl Attribute {

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn value(&self) -> &Value {
        &self.value
    }

    pub fn is_type<T: 'static>(&self) -> bool {
        match self.value() {
            Value::Type(ty) => ty.same_as::<T>(),
            _ => false,
        }
    }

    pub fn is_type_id(&self, value: &TypeId) -> bool {
        match self.value() {
            Value::Type(ty) => &ty.type_id() == value,
            _ => false,
        }
    }

    pub const fn is_str(&self, value: &str) -> bool {
        match self.value() {
            Value::Str(string) => const_str_equal(string, value),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Type(Type),
    Str(&'static str),
    Bool(bool),
    Int(i32),
}

#[derive(Debug, Clone)]
pub struct Type {
    pub(crate) type_name_fn: fn() -> &'static str,
    pub(crate) type_id: TypeId,
}

impl Type {

    pub fn type_name(&self) -> &'static str {
        (self.type_name_fn)()
    }

    pub const fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub fn same_as<T: 'static>(&self) -> bool {
        TypeId::of::<T>() == self.type_id()
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.type_id() == other.type_id()
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Attributes(&'static [Attribute]);

impl Attributes {
    pub (crate) const fn new(attributes: &'static [Attribute]) -> Attributes {
        Attributes(attributes)
    }
    
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub const fn named(&self, name: &str) -> Option<&Attribute> {
        let mut index = 0;
        while index < self.0.len() {
            let attribute = &self.0[index];
            if const_str_equal(attribute.name(), name) {
                return Some(attribute);
            }
            index += 1;
        }
        None
    }
    pub fn iter(&self) -> Iter<'_, Attribute> {
        self.0.iter()
    }
}

impl Index<usize> for Attributes {
    type Output = Attribute;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IntoIterator for Attributes {
    type Item = &'static Attribute;
    type IntoIter = Iter<'static, Attribute>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
