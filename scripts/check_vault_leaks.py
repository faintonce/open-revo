"""Check hardware literals and address-based names outside the two HAL roots."""
import argparse
import json
import pathlib
import re
import sys

NUMBER = r"(?:0[xX][0-9a-fA-F_]+|0[bB][01_]+|0[oO][0-7_]+|[0-9][0-9_]*)(?:[ui](?:8|16|32|64|128|size))?"
TOKENS = re.compile(
    r'//[^\n]*|/\*.*?\*/|(?:br|cr|r)(?P<hashes>\#*)".*?"(?P=hashes)'
    r'|(?:b|c)?"(?:\\.|[^"\\])*"'
    r"|b?'(?:\\(?:u\{[^}]*\}|x[0-9a-fA-F]{2}|.)|[^'\\])'"
    r"|(?P<number>\b" + NUMBER + r"\b)", re.S,
)
EC_CALL = re.compile(r"\b\w*(?:read_ec|write_ec|ec_read|ec_write|ec_set|ec_update|ec_masked_write)\w*\s*\(\s*(?:&?\s*\[\s*\(?\s*)?(" + NUMBER + r")\b")
ADDRESS_NAME = re.compile(r"\b[A-Za-z_][A-Za-z_0-9]*\b")


def number_value(value):
    value = re.sub(r"[ui](?:8|16|32|64|128|size)$", "", value).replace("_", "")
    return int(value, 0 if value.lower().startswith(("0x", "0b", "0o")) else 10)


def non_address_number(before, after=""):
    # Timeouts, dimensions, memory sizes and RPM values can equal an EC address.
    return bool(
        re.search(r"(?:from_millis|from_secs|Next|WaitForSingleObject|fetch_single_manifest|clamp|setTimeout|setInterval)\s*\([^;{}]*$", before)
        or re.search(r"\b(?:width|height)\s*(?:[<>=!]=?|:)\s*$", before)
        or re.search(r"\b\w*vram\w*\b", before, re.I)
        or re.search(r"\bebx\s*(?:>=|<=)\s*$", before)
        or re.search(r"\b\w*(?:_ms|_secs|_seconds)\b", before)
        or re.search(r"\b(?:rpm|\w+_rpm(?:_per_sec)?|minRpm|maxRpm)\b", before + after)
    )


def scan_source(text, addresses, ioctls):
    problems = set()
    clean = list(text)
    for token in TOKENS.finditer(text):
        if token.group("number"):
            value = number_value(token.group("number"))
            before = text[text.rfind("\n", 0, token.start()) + 1:token.start()]
            line_end = text.find("\n", token.end())
            after = text[token.end():line_end if line_end >= 0 else len(text)]
            if value in ioctls or (value in addresses and not non_address_number(before, after)):
                problems.add((text.count("\n", 0, token.start()) + 1, "Hardware literal must use the vault: " + token.group("number")))
        else:
            for index in range(token.start(), token.end()):
                if clean[index] != "\n":
                    clean[index] = " "
    code = "".join(clean)
    for call in EC_CALL.finditer(code):
        problems.add((text.count("\n", 0, call.start()) + 1, "EC address argument must be symbolic"))
    for binding in re.finditer(r"\b(?:const|static|let)\s+(?:mut\s+)?(\w+)\s*(?::[^=;]+)?=\s*" + NUMBER + r"\b", code):
        if re.search(r"(?:ADDR|ADDRESS|REGISTER|IOCTL|EC_PORT|FRAME_HEADER|FRAME_TAIL|PACKET_HEADER|PACKET_SIZE)", binding[1], re.I):
            problems.add((text.count("\n", 0, binding.start()) + 1, "Hardware binding must use the vault: " + binding[1]))
    for field in re.finditer(r"\b(?:address|ec_addr|register)\s*:\s*" + NUMBER + r"\b", code):
        problems.add((text.count("\n", 0, field.start()) + 1, "Hardware address field must be symbolic"))
    for token in ADDRESS_NAME.finditer(text):
        name = token.group()
        parts = re.findall(r"[0-9]+", name)
        if any(int(part) in addresses and len(part) >= 3 for part in parts) or re.fullmatch(r"(?:cur_|pre_|r|reg_|addr_|ec_)\d{3,}", name):
            problems.add((text.count("\n", 0, token.start()) + 1, "Address-based name must be semantic: " + name))
    for frame in re.finditer(r"\b(?:const|static)\s+\w*(?:FRAME|PACKET)\w*\s*:\s*&?\s*\[[^\]]+\]\s*=\s*&?\s*\[\s*(?:" + NUMBER + r")\s*,", code):
        problems.add((text.count("\n", 0, frame.start()) + 1, "Fixed protocol frame must use the vault"))
    return sorted(problems)


def check(project_root):
    registry_path = project_root / "src-tauri/config/ec_registry.json"
    registry = json.loads(registry_path.read_text(encoding="utf-8-sig"))
    addresses = set(registry["ec"].values())
    ioctls = {value for key, value in registry["driver"].items() if key.startswith("IOCTL_")}
    if not addresses or any(type(value) is not int for value in addresses | ioctls):
        raise ValueError("Invalid hardware registry")
    total = 0
    for relative in ("src-tauri/src", "src-tauri/crates/openrevo-core/src", "src"):
        base = project_root / relative
        if not base.is_dir():
            raise ValueError(f"Source root not found: {base}")
        for path in sorted(base.rglob("*")):
            if path.suffix not in {".rs", ".ts", ".tsx", ".js", ".jsx"} or not path.is_file():
                continue
            if relative != "src" and path.relative_to(base).parts[0] == "hal":
                continue
            for line, problem in scan_source(path.read_text(encoding="utf-8-sig"), addresses, ioctls):
                print(f"{path.relative_to(project_root)}:{line}: {problem}")
                total += 1
    if total:
        print(f"Vault gate failed: {total} findings", file=sys.stderr)
        return 1
    print("Vault gate passed")
    return 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path(__file__).resolve().parent.parent)
    args = parser.parse_args()
    try:
        raise SystemExit(check(args.root.resolve(strict=True)))
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Vault gate could not run: {error}", file=sys.stderr)
        raise SystemExit(1)
