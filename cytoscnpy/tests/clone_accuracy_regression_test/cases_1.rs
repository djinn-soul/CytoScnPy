use super::*;

#[test]
fn curated_exact_clone_corpus_has_full_precision_and_recall() {
    let positives = [
        (
            "def render(values):\n    selected = [value * 2 for value in values if value > 0]\n    total = sum(selected)\n    message = f\"total={total}\"\n    return message\n",
            "def display(items):\n    chosen = [item * 2 for item in items if item > 0]\n    amount = sum(chosen)\n    text = f\"total={amount}\"\n    return text\n",
        ),
        (
            "def classify(value):\n    match value:\n        case [first, *rest] if first:\n            return first\n        case _:\n            return None\n",
            "def categorize(item):\n    match item:\n        case [head, *tail] if head:\n            return head\n        case _:\n            return None\n",
        ),
    ];
    let negatives = [
        (
            "def calculate(value):\n    first = value + 1\n    second = first + 2\n    third = second + 3\n    return third\n",
            "def calculate(value):\n    first = value - 1\n    second = first - 2\n    third = second - 3\n    return third\n",
        ),
        (
            "@cache\ndef load(value: int):\n    first = prepare(value, mode=True)\n    second = finalize(first)\n    return second\n",
            "@property\ndef load(value: str):\n    first = prepare(value, policy=True)\n    second = finalize(first)\n    return second\n",
        ),
        (
            "def unpack(value):\n    match value:\n        case [first, *rest]:\n            return first\n        case _:\n            return None\n",
            "def unpack(value):\n    match value:\n        case {\"first\": first, **rest}:\n            return first\n        case _:\n            return None\n",
        ),
    ];

    let true_positives = positives
        .iter()
        .filter(|(left, right)| detects_pair(left, right))
        .count();
    let false_positives = negatives
        .iter()
        .filter(|(left, right)| detects_pair(left, right))
        .count();

    assert_eq!(true_positives, positives.len(), "curated recall regressed");
    assert_eq!(false_positives, 0, "curated precision regressed");
}

#[test]
fn enum_member_names_and_values_remain_semantic() {
    let payment_states = "from enum import Enum\n\nclass PaymentState(Enum):\n    PENDING = \"pending\"\n    SETTLED = \"settled\"\n    FAILED = \"failed\"\n    REFUNDED = \"refunded\"\n";
    let access_levels = "from enum import Enum\n\nclass AccessLevel(Enum):\n    GUEST = \"guest\"\n    MEMBER = \"member\"\n    ADMIN = \"admin\"\n    OWNER = \"owner\"\n";
    let renamed_payment_states = "from enum import Enum\n\nclass TransactionState(Enum):\n    PENDING = 'pending'\n    SETTLED = 'settled'\n    FAILED = 'failed'\n    REFUNDED = 'refunded'\n";

    assert!(
        !detects_pair(payment_states, access_levels),
        "different enum members must not be reported as clones"
    );
    assert!(
        detects_pair(payment_states, renamed_payment_states),
        "renaming only the enum class should remain a Type-2 clone"
    );
}

#[test]
fn qualified_enum_bases_preserve_member_semantics() {
    let colors = "import enum\n\nclass Color(enum.IntEnum):\n    RED = 1\n    GREEN = 2\n    BLUE = 3\n    WHITE = 4\n";
    let priorities = "import enum\n\nclass Priority(enum.IntEnum):\n    LOW = 10\n    MEDIUM = 20\n    HIGH = 30\n    URGENT = 40\n";

    assert!(
        !detects_pair(colors, priorities),
        "qualified enum bases must preserve member semantics"
    );
}

#[test]
fn custom_base_enum_classes_are_not_false_positive_clones() {
    // BaseEnum inherits from Enum — suffix heuristic should catch it.
    let task_status = "from app.enums import BaseEnum\n\nclass TaskStatus(BaseEnum):\n    OPEN = \"open\"\n    IN_PROGRESS = \"in_progress\"\n    DONE = \"done\"\n    CANCELLED = \"cancelled\"\n";
    let user_role = "from app.enums import BaseEnum\n\nclass UserRole(BaseEnum):\n    VIEWER = \"viewer\"\n    EDITOR = \"editor\"\n    ADMIN = \"admin\"\n    OWNER = \"owner\"\n";

    assert!(
        !detects_pair(task_status, user_role),
        "custom base enum subclasses must not be false-positive clones"
    );
}

#[test]
fn framework_enum_classes_are_not_false_positive_clones() {
    let status_choices = "from django.db import models\n\nclass StatusChoices(models.TextChoices):\n    DRAFT = \"draft\"\n    PUBLISHED = \"published\"\n    ARCHIVED = \"archived\"\n    DELETED = \"deleted\"\n";
    let priority_choices = "from django.db import models\n\nclass PriorityChoices(models.TextChoices):\n    LOW = \"low\"\n    MEDIUM = \"medium\"\n    HIGH = \"high\"\n    CRITICAL = \"critical\"\n";

    assert!(
        !detects_pair(status_choices, priority_choices),
        "Django TextChoices must not be false-positive clones"
    );
}

#[test]
fn repr_enum_subclasses_are_not_false_positive_clones() {
    let shapes = "from enum import ReprEnum\n\nclass Shape(ReprEnum):\n    CIRCLE = 1\n    SQUARE = 2\n    TRIANGLE = 3\n    HEXAGON = 4\n";
    let sizes = "from enum import ReprEnum\n\nclass Size(ReprEnum):\n    SMALL = 10\n    MEDIUM = 20\n    LARGE = 30\n    XLARGE = 40\n";

    assert!(
        !detects_pair(shapes, sizes),
        "ReprEnum subclasses must not be false-positive clones"
    );
}

#[test]
fn auto_enum_subclasses_are_not_false_positive_clones() {
    let directions = "from enum import Enum, auto\n\nclass Direction(Enum):\n    NORTH = auto()\n    SOUTH = auto()\n    EAST = auto()\n    WEST = auto()\n";
    let seasons = "from enum import Enum, auto\n\nclass Season(Enum):\n    SPRING = auto()\n    SUMMER = auto()\n    AUTUMN = auto()\n    WINTER = auto()\n";

    assert!(
        !detects_pair(directions, seasons),
        "auto() enums must not be false-positive clones"
    );
}

#[test]
fn overlapping_enum_members_are_not_false_positive_clones() {
    // Enums that share structure and some member names but differ on others.
    let http_success = "from enum import IntEnum\n\nclass HttpSuccess(IntEnum):\n    OK = 200\n    CREATED = 201\n    ACCEPTED = 202\n    NO_CONTENT = 204\n    RESET = 205\n";
    let http_redirect = "from enum import IntEnum\n\nclass HttpRedirect(IntEnum):\n    MOVED = 301\n    FOUND = 302\n    SEE_OTHER = 303\n    NOT_MODIFIED = 304\n    TEMPORARY = 307\n";

    assert!(
        !detects_pair(http_success, http_redirect),
        "partially overlapping enum members must not be false-positive clones"
    );
}

#[test]
fn pydantic_models_with_different_fields_are_not_false_positive_clones() {
    let user_model = "from pydantic import BaseModel\n\nclass UserCreate(BaseModel):\n    name: str\n    email: str\n    age: int\n    active: bool\n";
    let product_model = "from pydantic import BaseModel\n\nclass ProductCreate(BaseModel):\n    title: str\n    price: float\n    quantity: int\n    available: bool\n";

    assert!(
        !detects_pair(user_model, product_model),
        "Pydantic models with different field names must not be false-positive clones"
    );
}

#[test]
fn pydantic_models_with_defaults_are_not_false_positive_clones() {
    let config_a = "from pydantic import BaseModel\n\nclass DatabaseConfig(BaseModel):\n    host: str = \"localhost\"\n    port: int = 5432\n    timeout: int = 30\n    ssl: bool = True\n";
    let config_b = "from pydantic import BaseModel\n\nclass CacheConfig(BaseModel):\n    url: str = \"redis://localhost\"\n    ttl: int = 3600\n    max_size: int = 100\n    enabled: bool = True\n";

    assert!(
        !detects_pair(config_a, config_b),
        "Pydantic models with different field names and defaults must not be false-positive clones"
    );
}

#[test]
fn django_model_classes_are_not_false_positive_clones() {
    let article = "from django.db import models\n\nclass Article(models.Model):\n    title: str\n    slug: str\n    published: bool\n    views: int\n";
    let comment = "from django.db import models\n\nclass Comment(models.Model):\n    author: str\n    body: str\n    approved: bool\n    score: int\n";

    assert!(
        !detects_pair(article, comment),
        "Django Model subclasses with different fields must not be false-positive clones"
    );
}

#[test]
fn custom_base_model_subclasses_are_not_false_positive_clones() {
    // AppBaseModel inherits from BaseModel — suffix heuristic catches it.
    let request = "from app.base import AppBaseModel\n\nclass OrderRequest(AppBaseModel):\n    customer_id: str\n    items: list\n    total: float\n    currency: str\n";
    let response = "from app.base import AppBaseModel\n\nclass OrderResponse(AppBaseModel):\n    order_id: str\n    status: str\n    created_at: float\n    tracking: str\n";

    assert!(
        !detects_pair(request, response),
        "custom BaseModel subclasses with different fields must not be false-positive clones"
    );
}

#[test]
fn pydantic_models_with_model_config_are_not_false_positive_clones() {
    // model_config is class configuration, not a data field — should not
    // inflate similarity between structurally different models.
    let order = "from pydantic import BaseModel\nfrom pydantic import ConfigDict\nfrom typing import ClassVar\n\nclass OrderSchema(BaseModel):\n    model_config = ConfigDict(strict=True)\n    order_id: str\n    customer: str\n    total: float\n    shipped: bool\n";
    let invoice = "from pydantic import BaseModel\nfrom pydantic import ConfigDict\nfrom typing import ClassVar\n\nclass InvoiceSchema(BaseModel):\n    model_config = ConfigDict(strict=True)\n    invoice_id: str\n    vendor: str\n    amount: float\n    paid: bool\n";

    assert!(
        !detects_pair(order, invoice),
        "Pydantic models with model_config and different fields must not be false-positive clones"
    );
}
