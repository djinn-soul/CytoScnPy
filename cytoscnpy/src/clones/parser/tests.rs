#[cfg(test)]
mod tests {
    use super::super::{extract_subtrees, SubtreeType};
    use std::path::PathBuf;

    #[test]
    fn test_parser_async_function() {
        let source = "
async def fetch_data():
    x = await api.get()
    y = x + 1
    return x
";
        let subtrees = extract_subtrees(source, &PathBuf::from("test.py")).unwrap();

        assert_eq!(subtrees.len(), 1);
        assert!(
            matches!(subtrees[0].node_type, SubtreeType::AsyncFunction),
            "Expected AsyncFunction, got {:?}",
            subtrees[0].node_type
        );
        assert_eq!(subtrees[0].name.as_deref(), Some("fetch_data"));
    }

    #[test]
    fn test_parser_nested_function() {
        let source = "
def outer():
    def inner():
        x = 1
        y = 2
        return x + y
    return inner
";
        let subtrees = extract_subtrees(source, &PathBuf::from("test.py")).unwrap();

        assert_eq!(subtrees.len(), 2);
        let names: Vec<&str> = subtrees.iter().filter_map(|s| s.name.as_deref()).collect();
        assert!(names.contains(&"outer"));
        assert!(names.contains(&"inner"));
    }

    #[test]
    fn test_parser_inner_class() {
        let source = "
def factory():
    class Local:
        def helper(self):
            x = 1
            return x
    return Local
";
        let subtrees = extract_subtrees(source, &PathBuf::from("test.py")).unwrap();

        assert_eq!(subtrees.len(), 2);
        assert!(subtrees
            .iter()
            .any(|s| s.node_type == SubtreeType::Function));
        assert!(subtrees.iter().any(|s| s.node_type == SubtreeType::Class));
    }

    #[test]
    fn test_parser_async_method() {
        let source = "
class API:
    async def get(self):
        x = 1
        y = 2
        return x
";
        let subtrees = extract_subtrees(source, &PathBuf::from("test.py")).unwrap();

        assert_eq!(subtrees.len(), 2);

        let method = subtrees
            .iter()
            .find(|s| s.name.as_deref() == Some("get"))
            .unwrap();
        assert_eq!(method.node_type, SubtreeType::Method);
    }
}
