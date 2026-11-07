use super::*;

#[test]
fn test_metaclass() {
    let project = TestProject::new();
    project
        .create_file(
            "metaclass.py",
            r#"
class SingletonMeta(type):
    _instances = {}

    def __call__(cls, *args, **kwargs):
        if cls not in cls._instances:
            instance = super().__call__(*args, **kwargs)
            cls._instances[cls] = instance
        return cls._instances[cls]

class Database(metaclass=SingletonMeta):
    def __init__(self):
        self.connection = None

    def connect(self):
        self.connection = "connected"
        return self.connection

db = Database()
db.connect()
"#,
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}

#[test]
fn test_iterator_protocol() {
    let project = TestProject::new();
    project
        .create_file(
            "iterators.py",
            r"
class CountUp:
    def __init__(self, max):
        self.max = max
        self.current = 0

    def __iter__(self):
        return self

    def __next__(self):
        if self.current < self.max:
            self.current += 1
            return self.current
        raise StopIteration

counter = CountUp(5)
result = list(counter)
",
        )
        .expect("Failed to create test file");

    assert!(project.path().exists());
}
