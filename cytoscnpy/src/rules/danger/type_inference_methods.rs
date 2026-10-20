//! Known method names of Python builtin types, used to flag method misuse.

const PROTOCOL_METHODS: [&str; 9] = [
    "__len__",
    "__iter__",
    "__contains__",
    "__str__",
    "__repr__",
    "__eq__",
    "__ne__",
    "__hash__",
    "__bool__",
];

fn is_valid_str_method(method_name: &str) -> bool {
    matches!(
        method_name,
        "capitalize"
            | "casefold"
            | "center"
            | "count"
            | "encode"
            | "endswith"
            | "expandtabs"
            | "find"
            | "format"
            | "format_map"
            | "index"
            | "isalnum"
            | "isalpha"
            | "isascii"
            | "isdecimal"
            | "isdigit"
            | "isidentifier"
            | "islower"
            | "isnumeric"
            | "isprintable"
            | "isspace"
            | "istitle"
            | "isupper"
            | "join"
            | "ljust"
            | "lower"
            | "lstrip"
            | "maketrans"
            | "partition"
            | "removeprefix"
            | "removesuffix"
            | "replace"
            | "rfind"
            | "rindex"
            | "rjust"
            | "rpartition"
            | "rsplit"
            | "rstrip"
            | "split"
            | "splitlines"
            | "startswith"
            | "strip"
            | "swapcase"
            | "title"
            | "translate"
            | "upper"
            | "zfill"
    )
}

fn is_valid_bytes_method(method_name: &str) -> bool {
    matches!(
        method_name,
        "capitalize"
            | "center"
            | "count"
            | "decode"
            | "endswith"
            | "expandtabs"
            | "find"
            | "fromhex"
            | "hex"
            | "index"
            | "isalnum"
            | "isalpha"
            | "isascii"
            | "isdigit"
            | "islower"
            | "isspace"
            | "istitle"
            | "isupper"
            | "join"
            | "ljust"
            | "lower"
            | "lstrip"
            | "maketrans"
            | "partition"
            | "removeprefix"
            | "removesuffix"
            | "replace"
            | "rfind"
            | "rindex"
            | "rjust"
            | "rpartition"
            | "rsplit"
            | "rstrip"
            | "split"
            | "splitlines"
            | "startswith"
            | "strip"
            | "swapcase"
            | "title"
            | "translate"
            | "upper"
            | "zfill"
    )
}

fn is_valid_list_method(method_name: &str) -> bool {
    matches!(
        method_name,
        "append"
            | "clear"
            | "copy"
            | "count"
            | "extend"
            | "index"
            | "insert"
            | "pop"
            | "remove"
            | "reverse"
            | "sort"
    )
}

fn is_valid_dict_method(method_name: &str) -> bool {
    matches!(
        method_name,
        "clear"
            | "copy"
            | "fromkeys"
            | "get"
            | "items"
            | "keys"
            | "pop"
            | "popitem"
            | "setdefault"
            | "update"
            | "values"
    )
}

fn is_valid_set_method(method_name: &str) -> bool {
    matches!(
        method_name,
        "add"
            | "clear"
            | "copy"
            | "difference"
            | "difference_update"
            | "discard"
            | "intersection"
            | "intersection_update"
            | "isdisjoint"
            | "issubset"
            | "issuperset"
            | "pop"
            | "remove"
            | "symmetric_difference"
            | "symmetric_difference_update"
            | "union"
            | "update"
    )
}

fn is_valid_int_method(method_name: &str) -> bool {
    matches!(
        method_name,
        "bit_length"
            | "bit_count"
            | "to_bytes"
            | "from_bytes"
            | "as_integer_ratio"
            | "conjugate"
            | "real"
            | "imag"
    )
}

fn is_valid_float_method(method_name: &str) -> bool {
    matches!(
        method_name,
        "as_integer_ratio" | "is_integer" | "hex" | "fromhex" | "conjugate" | "real" | "imag"
    )
}

pub(super) fn is_valid_method(type_name: &str, method_name: &str) -> bool {
    if PROTOCOL_METHODS.contains(&method_name) {
        return true;
    }

    match type_name {
        "str" => is_valid_str_method(method_name),
        "bytes" => is_valid_bytes_method(method_name),
        "list" => is_valid_list_method(method_name),
        "tuple" => matches!(method_name, "count" | "index"),
        "dict" => is_valid_dict_method(method_name),
        "set" => is_valid_set_method(method_name),
        "int" => is_valid_int_method(method_name),
        "float" => is_valid_float_method(method_name),
        "bool" | "None" => false, // These don't have meaningful mutable methods
        _ => true,                // Unknown type, assume valid to reduce false positives
    }
}
