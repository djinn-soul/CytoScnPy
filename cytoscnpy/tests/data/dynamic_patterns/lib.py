def hidden_gem():
    """
    This function is not imported or called statically.
    It is accessed dynamically via globals() in another module.
    """
    return "You found me!"
