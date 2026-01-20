use super::utils::{create_finding, get_call_name};
use crate::rules::ids;
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

/// Rule for detecting insecure usage of `pickle` and similar deserialization modules.
pub const META_PICKLE: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_PICKLE,
    category: super::CAT_DESERIALIZATION,
};
/// Rule for detecting unsafe YAML loading.
pub const META_YAML: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_YAML,
    category: super::CAT_DESERIALIZATION,
};
/// Rule for detecting potentially dangerous `marshal.load()` calls.
pub const META_MARSHAL: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_MARSHAL,
    category: super::CAT_DESERIALIZATION,
};
/// Rule for detecting insecure deserialization of machine learning models.
pub const META_MODEL_DESER: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_MODEL_DESER,
    category: super::CAT_DESERIALIZATION,
};

/// Rule for detecting insecure usage of `pickle` and similar deserialization modules.
pub struct PickleRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl PickleRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for PickleRule {
    fn name(&self) -> &'static str {
        "PickleRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name.starts_with("pickle.")
                    || name.starts_with("cPickle.")
                    || name.starts_with("dill.")
                    || name.starts_with("shelve.")
                    || name.starts_with("jsonpickle.")
                    || name == "pandas.read_pickle")
                    && (name.contains("load")
                        || name.contains("Unpickler")
                        || name == "shelve.open"
                        || name == "shelve.DbfilenameShelf"
                        || name.contains("decode")
                        || name == "pandas.read_pickle")
                {
                    return Some(vec![create_finding(
                        "Avoid using pickle/dill/shelve/jsonpickle/pandas.read_pickle (vulnerable to RCE on untrusted data)",
                        self.metadata,
                        context,
                        call.range().start(),
                        "CRITICAL",
                    )]);
                }
            }
        }
        None
    }
}

/// Rule for detecting unsafe YAML loading.
pub struct YamlRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl YamlRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for YamlRule {
    fn name(&self) -> &'static str {
        "YamlRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "yaml.load" {
                    let mut is_safe = false;
                    for keyword in &call.arguments.keywords {
                        if let Some(arg) = &keyword.arg {
                            if arg == "Loader" {
                                if let Expr::Name(n) = &keyword.value {
                                    if n.id.as_str() == "SafeLoader" {
                                        is_safe = true;
                                    }
                                }
                            }
                        }
                    }
                    if !is_safe {
                        return Some(vec![create_finding(
                            "Use yaml.safe_load or Loader=SafeLoader",
                            self.metadata,
                            context,
                            call.range().start(),
                            "HIGH",
                        )]);
                    }
                }
            }
        }
        None
    }
}

/// Rule for detecting potentially dangerous `marshal.load()` calls.
pub struct MarshalRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl MarshalRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for MarshalRule {
    fn name(&self) -> &'static str {
        "MarshalRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "marshal.load" || name == "marshal.loads" {
                    return Some(vec![create_finding(
                        "Deserialization with marshal is insecure.",
                        self.metadata,
                        context,
                        call.range().start(),
                        "MEDIUM",
                    )]);
                }
            }
        }
        None
    }
}

/// Rule for detecting insecure deserialization of machine learning models.
pub struct ModelDeserializationRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl ModelDeserializationRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for ModelDeserializationRule {
    fn name(&self) -> &'static str {
        "ModelDeserializationRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "torch.load" {
                    let has_weights_only = call.arguments.keywords.iter().any(|kw| {
                        if let Some(arg) = &kw.arg {
                            if arg == "weights_only" {
                                if let Expr::BooleanLiteral(b) = &kw.value {
                                    return b.value;
                                }
                            }
                        }
                        false
                    });
                    if !has_weights_only {
                        return Some(vec![create_finding(
                            "torch.load() without weights_only=True can execute arbitrary code. Use weights_only=True or torch.safe_load().",
                            self.metadata,
                            context,
                            call.range().start(),
                            "CRITICAL",
                        )]);
                    }
                }

                if name == "joblib.load" {
                    return Some(vec![create_finding(
                        "joblib.load() can execute arbitrary code. Ensure the model source is trusted.",
                        self.metadata,
                        context,
                        call.range().start(),
                        "HIGH",
                    )]);
                }

                if name == "keras.models.load_model"
                    || name == "tf.keras.models.load_model"
                    || name == "load_model"
                    || name == "keras.load_model"
                {
                    let has_safe_mode = call.arguments.keywords.iter().any(|kw| {
                        if let Some(arg) = &kw.arg {
                            if arg == "safe_mode" {
                                if let Expr::BooleanLiteral(b) = &kw.value {
                                    return b.value;
                                }
                            }
                        }
                        false
                    });
                    if !has_safe_mode {
                        return Some(vec![create_finding(
                            "keras.models.load_model() without safe_mode=True can load Lambda layers with arbitrary code.",
                            self.metadata,
                            context,
                            call.range().start(),
                            "HIGH",
                        )]);
                    }
                }
            }
        }
        None
    }
}
