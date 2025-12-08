from .lib import hidden_gem

def runner():
    # Dynamic usage of the imported name
    # This should mark the module as dynamic, and thus 'hidden_gem' as used.
    g = globals()
    if "hidden_gem" in g:
        g["hidden_gem"]()
