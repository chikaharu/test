import re
from dataclasses import dataclass

FULL_TO_HALF = str.maketrans({"０":"0","１":"1","２":"2","３":"3","４":"4","５":"5","６":"6","７":"7","８":"8","９":"9","－":"-","ー":"-","―":"-"})
POSTAL_PATTERN = re.compile(r"(\d{3})-?(\d{4})")
ADDRESS_HAS_DIGIT_PATTERN = re.compile(r"[0-9０-９]")


@dataclass
class ValidationResult:
    status: str
    reason: str | None = None


def normalize_postal_code(value: str) -> str:
    normalized = value.translate(FULL_TO_HALF)
    match = POSTAL_PATTERN.search(normalized)
    if not match:
        return ""
    return f"{match.group(1)}-{match.group(2)}"


def validate_address(postal_code: str, address: str) -> ValidationResult:
    if not ADDRESS_HAS_DIGIT_PATTERN.search(address):
        return ValidationResult(status="hold", reason="address_missing_number")

    normalized_postal = normalize_postal_code(postal_code)
    address_postal = normalize_postal_code(address)

    if normalized_postal and address_postal and normalized_postal != address_postal:
        return ValidationResult(status="hold", reason="postal_address_mismatch")

    return ValidationResult(status="accepted")
