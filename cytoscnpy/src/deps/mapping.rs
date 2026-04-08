use rustc_hash::FxHashMap;
use std::sync::OnceLock;

/// Map of package names to their common import names
pub static PACKAGE_TO_IMPORT: OnceLock<FxHashMap<&'static str, Vec<&'static str>>> =
    OnceLock::new();
/// Reverse map of common import names to their package names
pub static IMPORT_TO_PACKAGE: OnceLock<FxHashMap<&'static str, &'static str>> = OnceLock::new();

/// Retrieve the mapping of package names to expected import names
pub fn get_package_mapping() -> &'static FxHashMap<&'static str, Vec<&'static str>> {
    PACKAGE_TO_IMPORT.get_or_init(|| {
        let mut map = FxHashMap::default();
        map.insert("pillow", vec!["PIL"]);
        map.insert("scikit_learn", vec!["sklearn"]);
        map.insert("pyyaml", vec!["yaml"]);
        map.insert("python_dateutil", vec!["dateutil"]);
        map.insert("beautifulsoup4", vec!["bs4"]);
        map.insert("python_dotenv", vec!["dotenv"]);
        map.insert("opencv_python", vec!["cv2"]);
        map.insert("opencv_python_headless", vec!["cv2"]);
        map.insert("apache_airflow", vec!["airflow"]);
        map.insert("pika", vec!["pika"]);
        map.insert("psycopg2_binary", vec!["psycopg2"]);
        map.insert("djangorestframework", vec!["rest_framework"]);
        map.insert("pyjwt", vec!["jwt"]);
        map.insert("pyspark", vec!["pyspark"]);
        map.insert("msgpack_python", vec!["msgpack"]);
        map.insert("pymongo", vec!["pymongo"]);
        map.insert("pygithub", vec!["github"]);
        map.insert("dnspython", vec!["dns"]);
        map.insert("attrs", vec!["attr", "attrs"]);
        map.insert("boto3", vec!["boto3"]);
        map.insert("celery", vec!["celery"]);
        map.insert("cryptography", vec!["cryptography"]);
        map.insert("flake8", vec!["flake8"]);
        map.insert("jinja2", vec!["jinja2"]);
        map.insert("markupsafe", vec!["markupsafe"]);
        map.insert("packaging", vec!["packaging"]);
        map.insert("pluggy", vec!["pluggy"]);
        map.insert("pyasn1", vec!["pyasn1"]);
        map.insert("pycparser", vec!["pycparser"]);
        map.insert("pygments", vec!["pygments"]);
        map.insert("pyparsing", vec!["pyparsing"]);
        map.insert("pytest", vec!["pytest"]);
        map.insert("pytz", vec!["pytz"]);
        map.insert("requests", vec!["requests"]);
        map.insert("six", vec!["six"]);
        map.insert("typing_extensions", vec!["typing_extensions"]);
        map.insert("urllib3", vec!["urllib3"]);
        map.insert("virtualenv", vec!["virtualenv"]);
        map.insert("werkzeug", vec!["werkzeug"]);
        map.insert("wrapt", vec!["wrapt"]);
        map.insert("zipp", vec!["zipp"]);
        map.insert("pylint", vec!["pylint"]);
        map.insert("scipy", vec!["scipy"]);
        map.insert("numpy", vec!["numpy"]);
        map.insert("pandas", vec!["pandas"]);
        map.insert("matplotlib", vec!["matplotlib"]);
        map.insert("seaborn", vec!["seaborn"]);
        map.insert("sqlalchemy", vec!["sqlalchemy"]);
        map.insert("fastapi", vec!["fastapi"]);
        map.insert("flask", vec!["flask"]);
        map.insert("django", vec!["django"]);
        map.insert("pydantic", vec!["pydantic"]);
        map
    })
}

/// Retrieve the reverse mapping from import name back to the package name
pub fn get_reverse_mapping() -> &'static FxHashMap<&'static str, &'static str> {
    IMPORT_TO_PACKAGE.get_or_init(|| {
        let mut map = FxHashMap::default();
        for (pkg, imports) in get_package_mapping() {
            for imp in imports {
                map.insert(*imp, *pkg);
            }
        }
        map
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_mapping_contains_common_mismatches() {
        let mapping = get_package_mapping();
        assert_eq!(mapping.get("pillow").unwrap(), &vec!["PIL"]);
        assert_eq!(mapping.get("scikit_learn").unwrap(), &vec!["sklearn"]);
        assert_eq!(mapping.get("pyyaml").unwrap(), &vec!["yaml"]);
    }

    #[test]
    fn test_reverse_mapping() {
        let reverse = get_reverse_mapping();
        assert_eq!(reverse.get("PIL").unwrap(), &"pillow");
        assert_eq!(reverse.get("sklearn").unwrap(), &"scikit_learn");
        assert_eq!(reverse.get("yaml").unwrap(), &"pyyaml");
    }
}
