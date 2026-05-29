#!/usr/bin/env python3
"""
Pre-deployment configuration validator for AnchorKit
Validates TOML/JSON configs against strict schema before runtime
"""

import json
import sys
from pathlib import Path
from typing import Dict, List, Any

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
    
    if not role or len(role) > 32:
        raise ValidationError(f"Attestor {index} ({name}): role must be 1-32 chars")

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

def main():
    # Use pathlib for cross-platform path handling
    script_dir = Path(__file__).resolve().parent
    config_dir = script_dir.joinpath("configs")
    
    if not config_dir.exists():
        print(f"Error: configs directory not found at {config_dir}")
        sys.exit(1)
    
    json_files = list(config_dir.glob("*.json"))
    
    if not json_files:
        print("No JSON config files found")
        sys.exit(1)
    
    errors = []
    
    for config_file in json_files:
        try:
            validate_json_config(config_file)
        except ValidationError as e:
            errors.append(f"{config_file.name}: {e}")
        except Exception as e:
            errors.append(f"{config_file.name}: Unexpected error - {e}")
    
    if errors:
        print("\n❌ Validation failed:\n")
        for error in errors:
            print(f"  • {error}")
        sys.exit(1)
    
    print(f"\n✅ All {len(json_files)} configuration files are valid")

if __name__ == "__main__":
    main()
