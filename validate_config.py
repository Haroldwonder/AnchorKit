#!/usr/bin/env python3
"""
Pre-deployment configuration validator for AnchorKit
Validates TOML/JSON configs against strict schema before runtime
"""

import json
import sys
from pathlib import Path
from typing import Dict, List, Any

# tomllib is in the standard library from Python 3.11+.
# For older Pythons install the backport: pip install tomli
try:
    import tomllib  # type: ignore[import]
except ModuleNotFoundError:
    try:
        import tomli as tomllib  # type: ignore[import,no-redef]
    except ModuleNotFoundError:
        tomllib = None  # type: ignore[assignment]

class ValidationError(Exception):
    pass

def validate_required_fields(config: Dict[str, Any]) -> None:
    """Validate the required deployment configuration structure."""
    if "contract" not in config:
        raise ValidationError("Missing required top-level section: contract")

    if "attestors" not in config:
        raise ValidationError("Missing required top-level section: attestors")

    if "sessions" not in config:
        raise ValidationError("Missing required top-level section: sessions")

    contract = config["contract"]
    for field in ("name", "version", "network"):
        if field not in contract:
            raise ValidationError(f"Missing required contract field: {field}")

    attestors = config["attestors"]
    if "registry" not in attestors:
        raise ValidationError("Missing required attestor field: registry")

    registry = attestors["registry"]
    if not registry:
        raise ValidationError("Attestor registry cannot be empty")

    for index, attestor in enumerate(registry):
        for field in ("name", "address", "endpoint", "role", "enabled"):
            if field not in attestor:
                raise ValidationError(f"Attestor {index}: missing required field {field}")

    sessions = config["sessions"]
    for field in (
        "enable_session_tracking",
        "session_timeout_seconds",
        "operations_per_session",
        "audit_log_retention_days",
    ):
        if field not in sessions:
            raise ValidationError(f"Missing required session field: {field}")

def validate_contract_config(config: Dict[str, Any]) -> None:
    """Validate contract configuration"""
    name = config.get("name", "")
    version = config.get("version", "")
    network = config.get("network", "")
    
    if not name or len(name) > 64:
        raise ValidationError(f"Invalid contract name: must be 1-64 chars, got {len(name)}")
    
    if not version or len(version) > 16:
        raise ValidationError(f"Invalid version: must be 1-16 chars, got {len(version)}")
    
    if not network or len(network) > 32:
        raise ValidationError(f"Invalid network: must be 1-32 chars, got {len(network)}")

def validate_attestor(attestor: Dict[str, Any], index: int) -> None:
    """Validate single attestor configuration"""
    name = attestor.get("name", "")
    address = attestor.get("address", "")
    endpoint = attestor.get("endpoint", "")
    role = attestor.get("role", "")
    
    # Check for unknown/extra properties (additionalProperties: false in schema)
    allowed_fields = {"name", "address", "endpoint", "role", "enabled", "description"}
    extra_fields = set(attestor.keys()) - allowed_fields
    if extra_fields:
        raise ValidationError(f"Attestor {index}: unknown fields: {', '.join(sorted(extra_fields))}")
    
    if not name or len(name) > 64:
        raise ValidationError(f"Attestor {index}: invalid name length")
    
    # Stellar addresses are typically 56 chars, but allow 54-56 for flexibility
    if len(address) < 54 or len(address) > 56:
        raise ValidationError(f"Attestor {index} ({name}): address must be 54-56 chars, got {len(address)}")
    
    if not address.startswith("G"):
        raise ValidationError(f"Attestor {index} ({name}): Stellar address must start with 'G'")
    
    if len(endpoint) < 8 or len(endpoint) > 256:
        raise ValidationError(f"Attestor {index} ({name}): endpoint must be 8-256 chars")
    
    if not endpoint.startswith(("http://", "https://")):
        raise ValidationError(f"Attestor {index} ({name}): endpoint must start with http:// or https://")
    
    # Validate role against enum from config_schema.json
    valid_roles = {"kyc-issuer", "transfer-verifier", "compliance-approver", "rate-provider", "attestor"}
    if not role:
        raise ValidationError(f"Attestor {index} ({name}): role is required")
    if role not in valid_roles:
        raise ValidationError(f"Attestor {index} ({name}): role must be one of {sorted(valid_roles)}, got '{role}'")

def validate_attestors(attestors: List[Dict[str, Any]]) -> None:
    """Validate attestor registry"""
    if not attestors:
        raise ValidationError("Attestor registry cannot be empty")
    
    if len(attestors) > 100:
        raise ValidationError(f"Too many attestors: max 100, got {len(attestors)}")
    
    for idx, attestor in enumerate(attestors):
        validate_attestor(attestor, idx)

def validate_session_config(config: Dict[str, Any]) -> None:
    """Validate session configuration"""
    timeout = config.get("session_timeout_seconds", 0)
    max_ops = config.get("operations_per_session", 0)
    
    if timeout <= 0 or timeout > 86400:
        raise ValidationError(f"Invalid session timeout: must be 1-86400 seconds, got {timeout}")
    
    if max_ops <= 0 or max_ops > 10000:
        raise ValidationError(f"Invalid max operations: must be 1-10000, got {max_ops}")

def validate_json_config(file_path: Path) -> None:
    """Validate JSON configuration file"""
    print(f"Validating {file_path}...")
    
    with open(file_path) as f:
        config = json.load(f)

    validate_required_fields(config)
    
    if "contract" in config:
        validate_contract_config(config["contract"])
    
    if "attestors" in config and "registry" in config["attestors"]:
        validate_attestors(config["attestors"]["registry"])
    
    if "sessions" in config:
        validate_session_config(config["sessions"])
    
    print(f"✓ {file_path.name} is valid")


def validate_toml_config(file_path: Path) -> None:
    """Validate TOML configuration file (same rules as JSON)."""
    if tomllib is None:
        raise ValidationError(
            "TOML support requires Python 3.11+ (tomllib) or the 'tomli' package. "
            "Install it with: pip install tomli"
        )

    print(f"Validating {file_path}...")

    with open(file_path, "rb") as f:
        config = tomllib.load(f)

    validate_required_fields(config)

    if "contract" in config:
        validate_contract_config(config["contract"])

    if "attestors" in config and "registry" in config["attestors"]:
        validate_attestors(config["attestors"]["registry"])

    if "sessions" in config:
        validate_session_config(config["sessions"])

    print(f"✓ {file_path.name} is valid")


def main():
    # Allow the caller (e.g. `anchorkit config validate <path>`) to override
    # the default configs/ directory via an environment variable.
    import os
    env_path = os.environ.get("ANCHORKIT_CONFIG_PATH")

    script_dir = Path(__file__).resolve().parent

    if env_path:
        target = Path(env_path)
        # Relative paths are resolved against the CWD of the caller.
        if not target.is_absolute():
            target = Path.cwd() / target
    else:
        target = script_dir / "configs"

    if target.is_file():
        # Single file mode
        if target.suffix == ".json":
            json_files = [target]
            toml_files: List[Path] = []
        elif target.suffix == ".toml":
            json_files = []
            toml_files = [target]
        else:
            print(f"Error: {target} is not a JSON or TOML file")
            sys.exit(1)
    elif target.is_dir():
        json_files = list(target.glob("*.json"))
        toml_files = list(target.glob("*.toml"))
    else:
        print(f"Error: path not found: {target}")
        sys.exit(1)

    all_files = json_files + toml_files
    if not all_files:
        print("No JSON or TOML config files found")
        sys.exit(1)

    errors = []

    for config_file in json_files:
        try:
            validate_json_config(config_file)
        except ValidationError as e:
            errors.append(f"{config_file.name}: {e}")
        except Exception as e:
            errors.append(f"{config_file.name}: Unexpected error - {e}")

    for config_file in toml_files:
        try:
            validate_toml_config(config_file)
        except ValidationError as e:
            errors.append(f"{config_file.name}: {e}")
        except Exception as e:
            errors.append(f"{config_file.name}: Unexpected error - {e}")

    if errors:
        print("\n❌ Validation failed:\n")
        for error in errors:
            print(f"  • {error}")
        sys.exit(1)

    print(f"\n✅ All {len(all_files)} configuration file(s) are valid")

if __name__ == "__main__":
    main()
