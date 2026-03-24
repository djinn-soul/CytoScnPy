use rustc_hash::FxHashMap;
use std::sync::OnceLock;

/// Returns expected import names for a given normalized package name.
pub fn get_import_names(package_name: &str) -> Option<&'static [&'static str]> {
    static MAPPING: OnceLock<FxHashMap<&'static str, &'static [&'static str]>> = OnceLock::new();
    let map = MAPPING.get_or_init(|| {
        let mut m: FxHashMap<&'static str, &'static [&'static str]> = FxHashMap::default();
        // Common package to import mappings
        m.insert("beautifulsoup4", &["bs4"]);
        m.insert("pillow", &["PIL"]);
        m.insert("scikit_learn", &["sklearn"]);
        m.insert("pyyaml", &["yaml"]);
        m.insert("python_dateutil", &["dateutil"]);
        m.insert("python_dotenv", &["dotenv"]);
        m.insert("opencv_python", &["cv2"]);
        m.insert("opencv_python_headless", &["cv2"]);
        m.insert("psycopg2_binary", &["psycopg2"]);
        m.insert("pyjwt", &["jwt"]);
        m.insert("djangorestframework", &["rest_framework"]);
        m.insert("dnspython", &["dns"]);
        m.insert("google_api_python_client", &["googleapiclient"]);
        m.insert("mysql_connector_python", &["mysql"]);
        m.insert("pycrypto", &["Crypto"]);
        m.insert("pyopenssl", &["OpenSSL"]);
        m.insert("pyserial", &["serial"]);
        m.insert("ruamel_yaml", &["ruamel"]);
        m.insert("msgpack_python", &["msgpack"]);
        m.insert("typing_extensions", &["typing_extensions"]);
        m.insert("attrs", &["attr", "attrs"]);
        m.insert("paho_mqtt", &["paho"]);
        m.insert("pygobject", &["gi"]);
        m.insert("pywin32", &["win32api", "win32con", "win32com"]);
        m.insert("pyzmq", &["zmq"]);
        m.insert("sqlalchemy", &["sqlalchemy"]); // Mostly 1:1 but good to be explicit
        m
    });
    map.get(package_name).copied()
}

/// Returns the package name that provides the given imported module name.
pub fn get_package_name(import_name: &str) -> Option<&'static str> {
    static REVERSE: OnceLock<FxHashMap<&'static str, &'static str>> = OnceLock::new();
    let r_map = REVERSE.get_or_init(|| {
        let mut m = FxHashMap::default();
        m.insert("bs4", "beautifulsoup4");
        m.insert("PIL", "pillow");
        m.insert("sklearn", "scikit-learn");
        m.insert("yaml", "pyyaml");
        m.insert("dateutil", "python-dateutil");
        m.insert("dotenv", "python-dotenv");
        m.insert("cv2", "opencv-python");
        m.insert("psycopg2", "psycopg2-binary");
        m.insert("jwt", "pyjwt");
        m.insert("rest_framework", "djangorestframework");
        m.insert("dns", "dnspython");
        m.insert("googleapiclient", "google-api-python-client");
        m.insert("mysql", "mysql-connector-python");
        m.insert("Crypto", "pycrypto");
        m.insert("OpenSSL", "pyopenssl");
        m.insert("serial", "pyserial");
        m.insert("ruamel", "ruamel.yaml");
        m.insert("msgpack", "msgpack-python");
        m.insert("attr", "attrs");
        m.insert("paho", "paho-mqtt");
        m.insert("gi", "pygobject");
        m.insert("win32api", "pywin32");
        m.insert("win32con", "pywin32");
        m.insert("win32com", "pywin32");
        m.insert("zmq", "pyzmq");
        m
    });
    r_map.get(import_name).copied()
}
