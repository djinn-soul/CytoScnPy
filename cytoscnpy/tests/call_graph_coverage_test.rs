//! Call graph coverage tests.
use cytoscnpy::taint::call_graph::CallGraph;
use ruff_python_parser::parse_module;

#[test]
fn test_call_graph_complex_coverage() -> Result<(), Box<dyn std::error::Error>> {
    let source = r"
def target(): pass
def other(): pass

class MyClass:
    def method(self):
        target()

def complex_flow(x):
    # If
    if x:
        target()
    elif x > 1:
        other()
    else:
        target()
        
    # For
    for i in range(10):
        target()
    else:
        other()
        
    # While
    while x:
        target()
    
    # Try
    try:
        target()
    except ValueError:
        other()
    except:
        target()
    else:
        other()
    finally:
        target()
        
    # With
    with open('x') as f:
        target()
    
    # Return
    return target()

def complex_exprs():
    # BinOp
    x = target() + other()
    
    # IfExp
    y = target() if True else other()
    
    # List/Dict
    l = [target(), other()]
    d = {'k': target(), 'v': other()}
    
    # NamedExpr (walrus) - might not be supported but robust parsers handle it
    # (x := target())
    
def nested():
    def inner():
        target()
    inner()
";

    let parsed = parse_module(source).map_err(|e| format!("Parsing failed: {e:?}"))?;
    let module = parsed.into_syntax();

    let mut cg = CallGraph::new();
    cg.build_from_module(&module.body);

    // assert nodes exist
    assert!(cg.nodes.contains_key("complex_flow"));

    // assert edges exist
    let node = cg
        .nodes
        .get("complex_flow")
        .ok_or("Node 'complex_flow' not found")?;
    assert!(node.calls.contains("target"));
    assert!(node.calls.contains("other"));

    let expr_node = cg
        .nodes
        .get("complex_exprs")
        .ok_or("Node 'complex_exprs' not found")?;
    assert!(expr_node.calls.contains("target"));
    assert!(expr_node.calls.contains("other"));
    Ok(())
}
