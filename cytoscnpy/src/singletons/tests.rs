//! Unit tests for Python singleton pattern detection.
#![allow(clippy::unwrap_used)]

use std::path::Path;

use super::python::detect_python_singletons;
use super::types::SingletonKind;

fn py_path() -> &'static Path {
    Path::new("services.py")
}

#[test]
fn test_python_new_singleton_detected() {
    let src = concat!(
        "class DatabasePool:\n",
        "    _instance = None\n",
        "    def __new__(cls, *args, **kwargs):\n",
        "        if cls._instance is None:\n",
        "            cls._instance = super().__new__(cls)\n",
        "        return cls._instance\n",
    );
    let matches = detect_python_singletons(src, py_path());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].class_name, "DatabasePool");
    assert_eq!(matches[0].kind, SingletonKind::NewMethod);
    assert_eq!(matches[0].line, 1);
}

#[test]
fn test_python_get_instance_singleton_detected() {
    let src = concat!(
        "class AppConfig:\n",
        "    _instance = None\n",
        "    @classmethod\n",
        "    def get_instance(cls):\n",
        "        if cls._instance is None:\n",
        "            cls._instance = cls()\n",
        "        return cls._instance\n",
    );
    let matches = detect_python_singletons(src, py_path());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].class_name, "AppConfig");
    assert_eq!(matches[0].kind, SingletonKind::GetInstanceMethod);
}

#[test]
fn test_python_get_instance_camel_case() {
    let src = concat!(
        "class NetworkManager:\n",
        "    instance = None\n",
        "    @staticmethod\n",
        "    def getInstance():\n",
        "        return NetworkManager.instance\n",
    );
    let matches = detect_python_singletons(src, py_path());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].class_name, "NetworkManager");
    assert_eq!(matches[0].kind, SingletonKind::GetInstanceMethod);
}

#[test]
fn test_python_decorator_singleton_detected() {
    let src = concat!(
        "@singleton\n",
        "class CacheManager:\n",
        "    def __init__(self):\n",
        "        self.cache = {}\n",
    );
    let matches = detect_python_singletons(src, py_path());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].class_name, "CacheManager");
    assert_eq!(matches[0].kind, SingletonKind::Decorator);
}

#[test]
fn test_python_decorator_uppercase_singleton() {
    let src = concat!("@Singleton()\n", "class EventBus:\n", "    pass\n",);
    let matches = detect_python_singletons(src, py_path());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].class_name, "EventBus");
    assert_eq!(matches[0].kind, SingletonKind::Decorator);
}

#[test]
fn test_python_metaclass_singleton_detected() {
    let src = concat!(
        "class SessionStore(metaclass=Singleton):\n",
        "    def __init__(self):\n",
        "        self.sessions = {}\n",
    );
    let matches = detect_python_singletons(src, py_path());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].class_name, "SessionStore");
    assert_eq!(matches[0].kind, SingletonKind::Metaclass);
}

#[test]
fn test_python_metaclass_meta_suffix() {
    let src = concat!(
        "class AuthProvider(metaclass=SingletonMeta):\n",
        "    pass\n",
    );
    let matches = detect_python_singletons(src, py_path());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].class_name, "AuthProvider");
    assert_eq!(matches[0].kind, SingletonKind::Metaclass);
}

#[test]
fn test_regular_class_is_clean() {
    let src = concat!(
        "class User:\n",
        "    def __init__(self, name):\n",
        "        self.name = name\n",
        "    def get_name(self):\n",
        "        return self.name\n",
    );
    let matches = detect_python_singletons(src, py_path());
    assert!(matches.is_empty(), "regular class should not be flagged");
}

#[test]
fn test_multiple_singletons_in_file() {
    let src = concat!(
        "@singleton\n",
        "class One:\n",
        "    pass\n\n",
        "class Two(metaclass=Singleton):\n",
        "    pass\n",
    );
    let matches = detect_python_singletons(src, py_path());
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].class_name, "One");
    assert_eq!(matches[1].class_name, "Two");
}

#[test]
fn test_instance_field_without_singleton_method_not_flagged() {
    let src = concat!(
        "class Model:\n",
        "    _instance = None\n",
        "    def run(self):\n",
        "        pass\n",
    );
    let matches = detect_python_singletons(src, py_path());
    assert!(
        matches.is_empty(),
        "instance field alone without singleton accessor should not be flagged"
    );
}
