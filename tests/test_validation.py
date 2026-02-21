from validation import normalize_postal_code, validate_address


def test_normalize_postal_code_with_full_width_numbers() -> None:
    assert normalize_postal_code("１２３ー４５６７") == "123-4567"


def test_hold_when_address_has_no_number() -> None:
    result = validate_address("123-4567", "東京都千代田区丸の内")
    assert result.status == "hold"
    assert result.reason == "address_missing_number"


def test_hold_when_postal_code_mismatch() -> None:
    result = validate_address("123-4567", "〒999-0000 東京都千代田区丸の内1-1")
    assert result.status == "hold"
    assert result.reason == "postal_address_mismatch"


def test_accept_when_address_and_postal_are_valid() -> None:
    result = validate_address("123-4567", "〒123-4567 東京都千代田区丸の内1-1")
    assert result.status == "accepted"
    assert result.reason is None
