annotate::environment!();

#[pragma(value_tag = true)]
pub mod extensions {
    use annotate::*;

    #[pragma(associated_type = String)]
    mod module_with_associated_type {
        use std::path::Path;

        #[pragma]
        fn function_for_that_type(string: &str) -> usize {
            string.len()
        }

        #[pragma]
        fn function_for_another_type(path: &Path) -> usize {
            path.display().to_string().len()
        }
    }

    #[pragma(tag = "custom", active = true, value = 42)]
    fn pragma_with_attributes() -> String {
        "pragma_with_custom_tag".to_string()
    }

    #[pragma(tag = "typed", associated_type = String)]
    fn pragma_with_associated_type() {}

    #[pragma]
    fn count_items(a: Vec<i32>, b: &str) -> usize {
        a.len() + b.len()
    }

    #[pragma]
    fn get_item(a: &[String], index: usize) -> Option<&String> {
        a.get(index)
    }
}
