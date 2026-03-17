use core::fmt;
use core::fmt::{Debug, Display, Formatter};

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct Path(pub(crate) &'static [&'static str]);

impl Path {
    pub const fn segments(&self) -> &'static [&'static str] {
        self.0
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub const fn root(&self) -> &'static str {
        self.0[0]
    }

    pub const fn last(&self) -> Option<&'static str> {
        let len = self.0.len();
        if len == 0 {
            None
        } else {
            Some(self.0[len - 1])
        }
    }

    pub const fn all_but_last(&self) -> Self {
        let len = self.0.len();

        if len == 0 {
            Self(&[])
        } else {
            Self(self.0.split_at(len - 1).0)
        }
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut iter = self.0.iter();
        if let Some(item) = iter.next() {
            f.write_str(item)?;
        }
        for item in iter {
            f.write_str("::")?;
            f.write_str(item)?;
        }

        Ok(())
    }
}

impl PartialEq<str> for Path {
    fn eq(&self, other: &str) -> bool {
        other.split("::").eq(self.0.iter().copied())
    }
}

impl PartialEq<Path> for Path {
    fn eq(&self, other: &Path) -> bool {
        self.0 == other.0
    }
}
