# Category 7: Type Safety & Validation (CSP-D6xx)

Rules in this category detect method calls on objects whose types suggest a mismatch between expected and actual usage, or general type-related safety issues.

| Rule ID      | Pattern                  | Severity | Why it's risky                | Safer alternative / Fix          |
| :----------- | :----------------------- | :------- | :---------------------------- | :------------------------------- |
| **CSP-D601** | Type-based method misuse | **HIGH** | Logic errors / Type confusion | Use static typing and validation |

## In-depth: Method Misuse (CSP-D601)

CytoScnPy performs light-weight type inference to detect when methods are called on object types that don't support them, or when framework-specific methods are used in an unsafe context.

### Example

Calling a list-specific method like `append` on what appears to be a `dict` or a `None` value can lead to runtime crashes.

```python
def process_data(items):
    # If items can be None or a dict, this will crash
    items.append(42)
```

### Recommendation

Use `isinstance()` checks or Python type hints (`List[int]`) to ensure type safety.
