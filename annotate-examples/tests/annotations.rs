use annotate::Value;
use annotate_examples::environment;

#[test]
pub fn test_function_pragma_with_attributes() {
    let function = environment()
        .find_functions_such_that(&|function| function.name() == "pragma_with_attributes")
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(function.name(), "pragma_with_attributes");
    assert_eq!(
        function.path(),
        "annotate_examples::extensions::pragma_with_attributes"
    );

    let attributes = function.find_attributes_such_that(&|_| true);
    assert_eq!(attributes.len(), 3);

    let tag = function
        .find_attributes_such_that(&|attribute| attribute.name() == "tag")
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(tag.name(), "tag");
    assert_eq!(tag.value(), &Value::Str("custom"));

    let active = function
        .find_attributes_such_that(&|attribute| attribute.name() == "active")
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(active.name(), "active");
    assert_eq!(active.value(), &Value::Bool(true));

    let value = function
        .find_attributes_such_that(&|attribute| attribute.name() == "value")
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(value.name(), "value");
    assert_eq!(value.value(), &Value::Int(42));
}

#[test]
pub fn test_function_call_path_values() {
    let function = environment()
        .find_functions_such_that(&|function| function.name() == "count_items")
        .into_iter()
        .next()
        .unwrap();

    let result = function.call::<fn(Vec<i32>, &str) -> usize, _>(|f| f(vec![1, 2, 3], "world"));

    assert_eq!(result, 8);
}

#[test]
pub fn test_function_call_path_references() {
    let function = environment()
        .find_functions_such_that(&|function| function.name() == "get_item")
        .into_iter()
        .next()
        .unwrap();

    let items = vec!["Hello".to_string(), "World".to_string()];
    let result = function.call::<fn(&[String], usize) -> Option<&String>, _>(|f| f(&items, 0));

    assert_eq!(result, Some(&"Hello".to_string()));
}

#[test]
pub fn test_module_with_associated_type_string() {
    let module = environment()
        .find_modules_such_that(&|module| {
            module.has_attribute_such_that(&|attribute| {
                attribute.name() == "associated_type" && attribute.is_type::<String>()
            })
        })
        .into_iter()
        .next()
        .unwrap();

    let associated_types = module.find_attributes_such_that(&|attribute| {
        attribute.name() == "associated_type" && attribute.is_type::<String>()
    });
    assert_eq!(associated_types.len(), 1);

    assert_eq!(module.find_functions_such_that(&|_| true).len(), 2);

    let function = module
        .find_functions_such_that(&|function| function.same_as::<fn(&str) -> usize>())
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(function.name(), "function_for_that_type");

    let result = function.call::<fn(&str) -> usize, _>(|f| f("Hello"));
    assert_eq!(result, 5);
}
