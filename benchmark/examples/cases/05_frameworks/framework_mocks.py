"""Minimal framework-like mocks for benchmark corpus.

These keep benchmark files dependency-free while preserving decorator/DI patterns.
"""


class FastAPI:
    def include_router(self, _router):
        return None


class APIRouter:
    def get(self, _path):
        def decorator(func):
            return func

        return decorator

    def post(self, _path):
        def decorator(func):
            return func

        return decorator


def Depends(dependency):
    return dependency


def field_validator(_field_name):
    def decorator(func):
        return func

    return decorator


class BaseModel:
    pass


class Flask:
    def __init__(self, _name):
        pass

    def route(self, _path):
        def decorator(func):
            return func

        return decorator

    def register_blueprint(self, _bp, url_prefix=None):
        return url_prefix

    def run(self, debug=False):
        return debug


class Blueprint:
    def __init__(self, _name, _import_name):
        pass

    def route(self, _path):
        def decorator(func):
            return func

        return decorator
