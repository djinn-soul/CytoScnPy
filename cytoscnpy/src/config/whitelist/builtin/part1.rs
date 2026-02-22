use super::super::WhitelistEntry;

pub(super) fn entries() -> Vec<WhitelistEntry> {
    vec![
        // argparse - argument parser attributes
        WhitelistEntry {
            name: "add_argument".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "parse_args".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "parse_known_args".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "set_defaults".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "get_default".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "add_subparsers".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "add_parser".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "set_defaults".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        // logging - logger methods and attributes
        WhitelistEntry {
            name: "getLogger".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "debug".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "info".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "warning".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "error".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "critical".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "exception".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "log".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "addHandler".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "removeHandler".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "addFilter".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "removeFilter".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "propagate".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "setLevel".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "getEffectiveLevel".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        // threading - thread attributes
        WhitelistEntry {
            name: "is_alive".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "getName".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "setName".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "isDaemon".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "setDaemon".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "start".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "join".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "run".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        // enum - enum attributes
        WhitelistEntry {
            name: "name".into(),
            category: Some("enum".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "value".into(),
            category: Some("enum".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_value_".into(),
            category: Some("enum".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_name_".into(),
            category: Some("enum".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_missing_".into(),
            category: Some("enum".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_generate_next_value_".into(),
            category: Some("enum".into()),
            ..Default::default()
        },
        // ctypes - foreign function interface
        WhitelistEntry {
            name: "restype".into(),
            category: Some("ctypes".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "argtypes".into(),
            category: Some("ctypes".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "errcheck".into(),
            category: Some("ctypes".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "value".into(),
            category: Some("ctypes".into()),
            ..Default::default()
        },
        // socketserver - server attributes
        WhitelistEntry {
            name: "allow_reuse_address".into(),
            category: Some("socketserver".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "address_family".into(),
            category: Some("socketserver".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "socket_type".into(),
            category: Some("socketserver".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "request_queue_size".into(),
            category: Some("socketserver".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "timeout".into(),
            category: Some("socketserver".into()),
            ..Default::default()
        },
        // ssl - SSL context attributes
        WhitelistEntry {
            name: "check_hostname".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "verify_mode".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "protocol".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "options".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "load_cert_chain".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "load_verify_locations".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "set_ciphers".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "wrap_socket".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
    ]
}
