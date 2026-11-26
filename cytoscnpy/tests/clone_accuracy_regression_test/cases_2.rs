use super::*;

#[test]
fn pydantic_models_with_classvar_annotations_are_not_false_positive_clones() {
    let model_a = "from pydantic import BaseModel\nfrom typing import ClassVar\n\nclass TeamSettings(BaseModel):\n    _internal: ClassVar[str] = \"cached\"\n    team_name: str\n    max_members: int\n    region: str\n    active: bool\n";
    let model_b = "from pydantic import BaseModel\nfrom typing import ClassVar\n\nclass ProjectSettings(BaseModel):\n    _internal: ClassVar[str] = \"cached\"\n    project_name: str\n    max_tasks: int\n    category: str\n    public: bool\n";

    assert!(
        !detects_pair(model_a, model_b),
        "Pydantic models with ClassVar config and different fields must not be false-positive clones"
    );
}

#[test]
fn pydantic_step_data_and_organization_result_are_not_clones_at_default_similarity() {
    let step_data = "from pydantic import BaseModel, ConfigDict\n\nclass StepData(BaseModel):\n    model_config = ConfigDict(arbitrary_types_allowed=True)\n    step_id: int\n    step_name: str\n    status: str\n    payload: dict\n";
    let organization_result = "from pydantic import BaseModel, ConfigDict\n\nclass OrganizationResult(BaseModel):\n    model_config = ConfigDict(arbitrary_types_allowed=True)\n    org_id: int\n    org_title: str\n    country: str\n    metadata: dict\n";
    let step_data_copy = "from pydantic import BaseModel, ConfigDict\n\nclass StepDataCopy(BaseModel):\n    model_config = ConfigDict(arbitrary_types_allowed=True)\n    step_id: int\n    step_name: str\n    status: str\n    payload: dict\n";

    assert!(
        !detects_pair_at_similarity(step_data, organization_result, 0.8),
        "StepData and OrganizationResult must not be detected as clones at default similarity (0.8)"
    );
    assert!(
        detects_pair_at_similarity(step_data, step_data_copy, 0.8),
        "Identical Pydantic models with only class name changed must be detected as clones"
    );
}

#[test]
fn django_orm_plain_assignment_models_are_not_false_positive_clones() {
    let user_model = "from django.db import models\n\nclass UserProfile(models.Model):\n    name = models.CharField(max_length=100)\n    email = models.EmailField()\n    bio = models.TextField()\n    is_active = models.BooleanField(default=True)\n";
    let product_model = "from django.db import models\n\nclass ProductItem(models.Model):\n    title = models.CharField(max_length=100)\n    code = models.CharField(max_length=50)\n    desc = models.TextField()\n    in_stock = models.BooleanField(default=True)\n";

    assert!(
        !detects_pair_at_similarity(user_model, product_model, 0.8),
        "Django models with plain field assignments must not be clones"
    );
}

#[test]
fn abc_interfaces_with_abstract_methods_are_not_false_positive_clones() {
    let repo = "from abc import ABC, abstractmethod\n\nclass Repository(ABC):\n    @abstractmethod\n    def get(self, id: str):\n        pass\n    @abstractmethod\n    def save(self, entity):\n        pass\n    @abstractmethod\n    def delete(self, id: str):\n        pass\n    @abstractmethod\n    def list_all(self):\n        pass\n";
    let cache = "from abc import ABC, abstractmethod\n\nclass CacheService(ABC):\n    @abstractmethod\n    def fetch(self, key: str):\n        pass\n    @abstractmethod\n    def store(self, key, val):\n        pass\n    @abstractmethod\n    def evict(self, key: str):\n        pass\n    @abstractmethod\n    def clear_all(self):\n        pass\n";

    assert!(
        !detects_pair_at_similarity(repo, cache, 0.8),
        "ABC interfaces with stub methods must not be clones"
    );
}

#[test]
fn protocols_with_stub_methods_are_not_false_positive_clones() {
    let reader = "from typing import Protocol\n\nclass ReaderProtocol(Protocol):\n    def read(self, n: int) -> bytes:\n        ...\n    def close(self) -> None:\n        ...\n    def seek(self, offset: int) -> int:\n        ...\n";
    let writer = "from typing import Protocol\n\nclass WriterProtocol(Protocol):\n    def write(self, b: bytes) -> int:\n        ...\n    def flush(self) -> None:\n        ...\n    def truncate(self, size: int) -> int:\n        ...\n";

    assert!(
        !detects_pair_at_similarity(reader, writer, 0.8),
        "Protocol classes with stub methods must not be clones"
    );
}

#[test]
fn custom_exception_classes_are_not_false_positive_clones() {
    let user_err = "class UserNotFoundError(Exception):\n    def __init__(self, user_id: str):\n        super().__init__(f\"User {user_id} not found\")\n        self.user_id = user_id\n        self.code = 404\n";
    let order_err = "class OrderNotFoundError(Exception):\n    def __init__(self, order_id: str):\n        super().__init__(f\"Order {order_id} not found\")\n        self.order_id = order_id\n        self.code = 404\n";

    assert!(
        !detects_pair_at_similarity(user_err, order_err, 0.8),
        "Custom exception classes with standard __init__ must not be clones"
    );
}

#[test]
fn to_dict_and_from_dict_serialization_are_not_false_positive_clones() {
    let user_to_dict = "class User:\n    def to_dict(self):\n        return {\n            \"id\": self.id,\n            \"name\": self.name,\n            \"email\": self.email,\n            \"active\": self.active,\n        }\n";
    let product_to_dict = "class Product:\n    def to_dict(self):\n        return {\n            \"sku\": self.sku,\n            \"title\": self.title,\n            \"price\": self.price,\n            \"stock\": self.stock,\n        }\n";

    assert!(
        !detects_pair_at_similarity(user_to_dict, product_to_dict, 0.8),
        "to_dict serialization with different keys must not be clones"
    );

    let user_from_dict = "class User:\n    @classmethod\n    def from_dict(cls, data):\n        return cls(\n            id=data[\"id\"],\n            name=data[\"name\"],\n            email=data[\"email\"],\n            active=data[\"active\"],\n        )\n";
    let product_from_dict = "class Product:\n    @classmethod\n    def from_dict(cls, data):\n        return cls(\n            sku=data[\"sku\"],\n            title=data[\"title\"],\n            price=data[\"price\"],\n            stock=data[\"stock\"],\n        )\n";

    assert!(
        !detects_pair_at_similarity(user_from_dict, product_from_dict, 0.8),
        "from_dict factory methods with different keys must not be clones"
    );
}

#[test]
fn dataclass_classes_with_different_fields_are_not_false_positive_clones() {
    let point = "from dataclasses import dataclass\n\n@dataclass\nclass Point3D:\n    x: float\n    y: float\n    z: float\n    label: str\n";
    let color = "from dataclasses import dataclass\n\n@dataclass\nclass ColorRGBA:\n    r: int\n    g: int\n    b: int\n    a: float\n";

    assert!(
        !detects_pair_at_similarity(point, color, 0.8),
        "dataclasses with different fields must not be clones at default similarity (0.8)"
    );
}

#[test]
fn plain_dto_classes_with_different_fields_are_not_false_positive_clones() {
    let user_dto = "class UserDTO:\n    def __init__(self, user_id: str, email: str, name: str, active: bool):\n        self.user_id = user_id\n        self.email = email\n        self.name = name\n        self.active = active\n";
    let product_dto = "class ProductDTO:\n    def __init__(self, product_id: str, sku: str, title: str, in_stock: bool):\n        self.product_id = product_id\n        self.sku = sku\n        self.title = title\n        self.in_stock = in_stock\n";
    let user_dto_copy = "class UserDTOCopy:\n    def __init__(self, user_id: str, email: str, name: str, active: bool):\n        self.user_id = user_id\n        self.email = email\n        self.name = name\n        self.active = active\n";

    assert!(
        !detects_pair_at_similarity(user_dto, product_dto, 0.8),
        "Plain DTO classes with different constructor fields must not be clones"
    );
    assert!(
        detects_pair_at_similarity(user_dto, user_dto_copy, 0.8),
        "Identical DTO classes with only name changed must remain clones"
    );
}

#[test]
fn django_model_admin_classes() {
    let user_admin = "from django.contrib import admin\n\nclass UserAdmin(admin.ModelAdmin):\n    list_display = (\"username\", \"email\", \"is_staff\", \"is_active\")\n    search_fields = (\"username\", \"email\")\n    list_filter = (\"is_staff\", \"is_active\")\n    ordering = (\"-date_joined\",)\n";
    let product_admin = "from django.contrib import admin\n\nclass ProductAdmin(admin.ModelAdmin):\n    list_display = (\"title\", \"sku\", \"price\", \"is_available\")\n    search_fields = (\"title\", \"sku\")\n    list_filter = (\"is_available\",)\n    ordering = (\"-created_at\",)\n";

    assert!(
        !detects_pair_at_similarity(user_admin, product_admin, 0.8),
        "Django ModelAdmin classes with different field configurations must not be clones"
    );
}

#[test]
fn msgspec_struct_classes() {
    let user_struct = "import msgspec\n\nclass User(msgspec.Struct):\n    id: int\n    name: str\n    email: str\n    active: bool\n";
    let item_struct = "import msgspec\n\nclass Item(msgspec.Struct):\n    sku: str\n    title: str\n    price: float\n    stock: int\n";

    assert!(
        !detects_pair_at_similarity(user_struct, item_struct, 0.8),
        "msgspec.Struct classes with different fields must not be clones"
    );
}

#[test]
fn attrs_define_classes() {
    let creds = "from attrs import define\n\n@define\nclass UserCredentials:\n    username: str\n    password: str\n    token: str\n    expires_at: int\n";
    let db_cfg = "from attrs import define\n\n@define\nclass DatabaseConfig:\n    host: str\n    port: int\n    user: str\n    dbname: str\n";

    assert!(
        !detects_pair_at_similarity(creds, db_cfg, 0.8),
        "attrs @define classes with different fields must not be clones"
    );
}
