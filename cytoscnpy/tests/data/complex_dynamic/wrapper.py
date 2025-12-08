from .legacy_math import old_add

def run_legacy_code(code_str):
    # 'old_add' is imported but not used statically.
    # 'eval' makes this module dynamic.
    # Therefore, 'old_add' import is used.
    # Therefore, 'legacy_math.old_add' is used.
    return eval(code_str)
