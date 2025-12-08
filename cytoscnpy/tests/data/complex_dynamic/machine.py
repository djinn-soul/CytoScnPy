from .state_handlers import handle_state_start, handle_state_end

def run_state(state):
    # Globals access makes this module dynamic.
    # Imported functions 'handle_state_start' and 'handle_state_end' are marked used.
    # 'handle_state_unused' is NOT imported, so it remains unused.
    g = globals()
    func_name = "handle_state_" + state
    if func_name in g:
        g[func_name]()
