#!/usr/bin/env python3
"""
Configuration Validation Test Suite for AnchorKit
Tests parsing of TOML/JSON/env configs and validates:
- Missing required fields
- Invalid URLs
- Unsupported assets

Ensures invalid configs fail safely with proper error handling.
"""

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Dict, Any, List, Optional

# Try to import toml/tomllib, prefer built-in for Python 3.11+
try:
    import tomllib
    TOML_AVAILABLE = True
except ImportError:
    try:
        import toml
        TOML_AVAILABLE = True
    except ImportError:
        TOML_AVAILABLE = False

class ConfigValidationError(Exception):
    """Raised when configuration validation fails"""
    pass


class ConfigValidator:
    """Validates AnchorKit configuration files"""
    
    # Supported networks
    VALID_NETWORKS = ['stellar-testnet', 'stellar-mainnet', 'stellar-futurenet', 'stellar-public']
    
    # Supported roles for attestors
    VALID_ROLES = ['kyc-issuer', 'transfer-verifier', 'compliance-approver', 
                   'rate-provider', 'attestor', 'identity-verifier', 
                   'settlement-bank', 'corridor-manager', 'compliance-checker',
                   'reserve-verifier', 'collateral-custodian', 'treasury-operator',
                   'risk-analyst']
    
    # Supported asset types (for stablecoin configs)
    VALID_ASSET_TYPES = ['ETH', 'BTC', 'XLM', 'USD', 'EUR', 'GBP', 'USDC', 'USDT']
    
    # Supported currencies
    VALID_CURRENCIES = ['USD', 'EUR', 'GBP', 'JPY', 'MXN', 'NGN', 'PKR']
    
    def __init__(self, config: Dict[str, Any]):
        self.config = config
        self.errors: List[str] = []
        self.warnings: List[str] = []
    
    def validate(self) -> bool:
        """Run all validations and return True if valid"""
        self.errors = []
        self.warnings = []
        
        self._validate_contract()
        self._validate_attestors()
        self._validate_sessions()
        self._validate_assets()
        
        return len(self.errors) == 0
    
    def get_errors(self) -> List[str]:
        """Return list of validation errors"""
        return self.errors
    
    def get_warnings(self) -> List[str]:
        """Return list of validation warnings"""
        return self.warnings
    
    def _add_error(self, message: str):
        """Add an error message"""
        self.errors.append(message)
    
    def _add_warning(self, message: str):
        """Add a warning message"""
        self.warnings.append(message)
    
    def _validate_contract(self):
        """Validate contract section"""
        contract = self.config.get('contract')
        
        if contract is None:
            self._add_error("Missing required section: 'contract'")
            return
        
        # Validate name
        name = contract.get('name')
        if name is None:
            self._add_error("Missing required field: contract.name")
        elif not isinstance(name, str):
            self._add_error("contract.name must be a string")
        elif len(name) < 1 or len(name) > 64:
            self._add_error(f"contract.name must be 1-64 characters, got {len(name)}")
        elif not self._is_valid_name_format(name):
            self._add_error("contract.name must contain only lowercase letters, numbers, and hyphens")
        
        # Validate version
        version = contract.get('version')
        if version is None:
            self._add_error("Missing required field: contract.version")
        elif not isinstance(version, str):
            self._add_error("contract.version must be a string")
        elif not self._is_valid_version(version):
            self._add_error("contract.version must follow semantic versioning (e.g., 1.0.0)")
        
        # Validate network
        network = contract.get('network')
        if network is None:
            self._add_error("Missing required field: contract.network")
        elif network not in self.VALID_NETWORKS:
            self._add_error(f"contract.network must be one of: {', '.join(self.VALID_NETWORKS)}, got '{network}'")
    
    def _validate_attestors(self):
        """Validate attestors section"""
        attestors = self.config.get('attestors')
        
        if attestors is None:
            self._add_error("Missing required section: 'attestors'")
            return
        
        registry = attestors.get('registry')
        if registry is None:
            self._add_error("Missing required field: attestors.registry")
            return
        
        if not isinstance(registry, list):
            self._add_error("attestors.registry must be an array")
            return
        
        if len(registry) == 0:
            self._add_error("attestors.registry cannot be empty")
            return
        
        if len(registry) > 100:
            self._add_error(f"attestors.registry cannot exceed 100 items, got {len(registry)}")
        
        # Track names and addresses for duplicate checking
        names = []
        addresses = []
        
        for idx, attestor in enumerate(registry):
            self._validate_single_attestor(attestor, idx)
            
            name = attestor.get('name')
            address = attestor.get('address')
            
            if name:
                names.append(name)
            if address:
                addresses.append(address)
        
        # Check for duplicates
        duplicates = self._find_duplicates(names)
        if duplicates:
            self._add_error(f"Duplicate attestor names: {', '.join(duplicates)}")
        
        dup_addresses = self._find_duplicates(addresses)
        if dup_addresses:
            self._add_error(f"Duplicate attestor addresses: {', '.join(dup_addresses)}")
        
        # Check for at least one enabled attestor
        enabled = [a for a in registry if a.get('enabled', False)]
        if not enabled:
            self._add_error("At least one attestor must be enabled")
    
    def _validate_single_attestor(self, attestor: Dict[str, Any], index: int):
        """Validate a single attestor entry"""
        prefix = f"attestors.registry[{index}]"
        
        # Validate name
        name = attestor.get('name')
        if name is None:
            self._add_error(f"{prefix}: Missing required field 'name'")
        elif not isinstance(name, str):
            self._add_error(f"{prefix}.name must be a string")
        elif len(name) < 1 or len(name) > 64:
            self._add_error(f"{prefix}.name must be 1-64 characters")
        
        # Validate address
        address = attestor.get('address')
        if address is None:
            self._add_error(f"{prefix}: Missing required field 'address'")
        elif not isinstance(address, str):
            self._add_error(f"{prefix}.address must be a string")
        elif not self._is_valid_stellar_address(address):
            self._add_error(f"{prefix}.address is not a valid Stellar address")
        
        # Validate endpoint (optional but if present must be valid URL)
        endpoint = attestor.get('endpoint')
        if endpoint is not None:
            if not isinstance(endpoint, str):
                self._add_error(f"{prefix}.endpoint must be a string")
            elif not self._is_valid_url(endpoint):
                self._add_error(f"{prefix}.endpoint is not a valid URL")
        
        # Validate role
        role = attestor.get('role')
        if role is None:
            self._add_error(f"{prefix}: Missing required field 'role'")
        elif role not in self.VALID_ROLES:
            self._add_warning(f"{prefix}.role '{role}' is not a standard role")
    
    def _validate_sessions(self):
        """Validate sessions section"""
        sessions = self.config.get('sessions')
        
        if sessions is None:
            # Sessions are optional, just warn
            self._add_warning("Optional section 'sessions' not provided, using defaults")
            return
        
        timeout = sessions.get('session_timeout_seconds')
        if timeout is not None:
            if not isinstance(timeout, int):
                self._add_error("sessions.session_timeout_seconds must be an integer")
            elif timeout < 60:
                self._add_error(f"sessions.session_timeout_seconds must be at least 60, got {timeout}")
            elif timeout > 86400:
                self._add_error(f"sessions.session_timeout_seconds cannot exceed 86400, got {timeout}")
        
        max_ops = sessions.get('operations_per_session')
        if max_ops is not None:
            if not isinstance(max_ops, int):
                self._add_error("sessions.operations_per_session must be an integer")
            elif max_ops < 1:
                self._add_error(f"sessions.operations_per_session must be at least 1, got {max_ops}")
            elif max_ops > 10000:
                self._add_error(f"sessions.operations_per_session cannot exceed 10000, got {max_ops}")
    
    def _validate_assets(self):
        """Validate assets in various config sections"""
        
        # Check stablecoin collateral types
        stablecoin = self.config.get('stablecoin')
        if stablecoin:
            collateral_types = stablecoin.get('collateral_types', [])
            for idx, ct in enumerate(collateral_types):
                symbol = ct.get('symbol')
                if symbol and symbol not in self.VALID_ASSET_TYPES:
                    self._add_warning(f"stablecoin.collateral_types[{idx}].symbol '{symbol}' is not a standard asset")
        
        # Check compliance currencies
        compliance = self.config.get('compliance')
        if compliance:
            supported_currencies = compliance.get('supported_currencies', [])
            for currency in supported_currencies:
                if currency not in self.VALID_CURRENCIES:
                    self._add_warning(f"compliance.supported_currencies contains unsupported currency: {currency}")
    
    def _is_valid_name_format(self, name: str) -> bool:
        """Check if name contains only allowed characters"""
        import re
        return bool(re.match(r'^[a-z0-9-]+$', name))
    
    def _is_valid_version(self, version: str) -> bool:
        """Check if version follows semantic versioning"""
        import re
        return bool(re.match(r'^\d+\.\d+\.\d+$', version))
    
    def _is_valid_stellar_address(self, address: str) -> bool:
        """Validate Stellar address format"""
        if not address:
            return False
        if not address.startswith('G'):
            return False
        if len(address) < 54 or len(address) > 56:
            return False
        # Allow placeholder addresses (X, Y, Z used in example configs)
        # Check remaining characters are alphanumeric or +/
        return all(c.isalnum() or c in ['+', '/', 'X', 'Y', 'Z'] for c in address[1:])
    
    def _is_valid_url(self, url: str) -> bool:
        """Validate URL format"""
        import re
        if not url or len(url) < 8 or len(url) > 256:
            return False
        pattern = r'^https?://[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}(/.*)?$'
        return bool(re.match(pattern, url))
    
    def _find_duplicates(self, items: List[str]) -> List[str]:
        """Find duplicate items in a list"""
        seen = set()
        duplicates = set()
        for item in items:
            if item in seen:
                duplicates.add(item)
            seen.add(item)
        return list(duplicates)


def load_config(file_path: str) -> Dict[str, Any]:
    """Load configuration from file (TOML or JSON)"""
    path = Path(file_path)
    
    if not path.exists():
        raise ConfigValidationError(f"Config file not found: {file_path}")
    
    suffix = path.suffix.lower()
    
    if suffix == '.toml':
        if not TOML_AVAILABLE:
            raise ConfigValidationError("TOML parsing not available. Install with: pip install toml")
        with open(path, 'rb') as f:
            try:
                return tomllib.load(f)
            except NameError:
                # Fallback to toml library
                with open(path, 'r') as f_toml:
                    return toml.load(f_toml)
    elif suffix == '.json':
        with open(path, 'r') as f:
            return json.load(f)
    else:
        raise ConfigValidationError(f"Unsupported file format: {suffix}")


def validate_config_file(file_path: str) -> tuple[bool, List[str], List[str]]:
    """Validate a configuration file and return (is_valid, errors, warnings)"""
    try:
        config = load_config(file_path)
        validator = ConfigValidator(config)
        is_valid = validator.validate()
        return is_valid, validator.get_errors(), validator.get_warnings()
    except ConfigValidationError as e:
        return False, [str(e)], []
    except Exception as e:
        return False, [f"Unexpected error: {str(e)}"], []


# ============================================================
# TEST CASES
# ============================================================

class TestConfigValidation(unittest.TestCase):
    """Test cases for configuration validation"""
    
    def test_valid_toml_config(self):
        """Test that a valid TOML config parses correctly"""
        config = {
            "contract": {
                "name": "test-anchor",
                "version": "1.0.0",
                "network": "stellar-testnet"
            },
            "attestors": {
                "registry": [
                    {
                        "name": "test-attestor",
                        "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                        "endpoint": "https://example.com/verify",
                        "role": "attestor",
                        "enabled": True
                    }
                ]
            }
        }
        
        validator = ConfigValidator(config)
        is_valid = validator.validate()
        
        self.assertTrue(is_valid, f"Expected valid config but got errors: {validator.get_errors()}")
        self.assertEqual(len(validator.get_errors()), 0)
    
    def test_valid_json_config(self):
        """Test that a valid JSON config parses correctly"""
        config = {
            "contract": {
                "name": "json-anchor",
                "version": "1.0.0",
                "network": "stellar-mainnet"
            },
            "attestors": {
                "registry": [
                    {
                        "name": "json-attestor",
                        "address": "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
                        "endpoint": "https://json.example.com/verify",
                        "role": "kyc-issuer",
                        "enabled": True
                    }
                ]
            },
            "sessions": {
                "session_timeout_seconds": 3600,
                "operations_per_session": 1000
            }
        }
        
        validator = ConfigValidator(config)
        is_valid = validator.validate()
        
        self.assertTrue(is_valid, f"Expected valid config but got errors: {validator.get_errors()}")
    
    # Missing required fields tests
    def test_missing_contract_section(self):
        """Test that missing contract section fails validation"""
        config = {"attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "https://example.com", "role": "attestor", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("contract" in e.lower() for e in validator.get_errors()))
    
    def test_missing_contract_name(self):
        """Test that missing contract.name fails validation"""
        config = {"contract": {"version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "https://example.com", "role": "attestor", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("name" in e.lower() for e in validator.get_errors()))
    
    def test_missing_contract_version(self):
        """Test that missing contract.version fails validation"""
        config = {"contract": {"name": "test", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "https://example.com", "role": "attestor", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("version" in e.lower() for e in validator.get_errors()))
    
    def test_missing_contract_network(self):
        """Test that missing contract.network fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0"}, "attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "https://example.com", "role": "attestor", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("network" in e.lower() for e in validator.get_errors()))
    
    def test_missing_attestors_section(self):
        """Test that missing attestors section fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("attestors" in e.lower() for e in validator.get_errors()))
    
    def test_missing_attestor_name(self):
        """Test that missing attestor name fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "https://example.com", "role": "attestor", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("name" in e.lower() for e in validator.get_errors()))
    
    def test_missing_attestor_address(self):
        """Test that missing attestor address fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "endpoint": "https://example.com", "role": "attestor", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("address" in e.lower() for e in validator.get_errors()))
    
    def test_missing_attestor_role(self):
        """Test that missing attestor role fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "https://example.com", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("role" in e.lower() for e in validator.get_errors()))
    
    # Invalid URLs tests
    def test_invalid_endpoint_url_too_short(self):
        """Test that URL too short fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "http://a.b", "role": "attestor", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("url" in e.lower() for e in validator.get_errors()))
    
    def test_invalid_endpoint_url_no_protocol(self):
        """Test that URL without http/https fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "example.com/verify", "role": "attestor", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("url" in e.lower() for e in validator.get_errors()))
    
    def test_invalid_endpoint_url_wrong_format(self):
        """Test that URL with wrong format fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "not-a-url-at-all", "role": "attestor", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("url" in e.lower() for e in validator.get_errors()))
    
    # Unsupported assets tests
    def test_unsupported_collateral_asset(self):
        """Test that unsupported collateral asset generates warning"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "https://example.com", "role": "attestor", "enabled": True}]}, "stablecoin": {"collateral_types": [{"name": "unknown", "symbol": "UNSUPPORTED", "liquidation_ratio": 1.5}]}}
        validator = ConfigValidator(config)
        is_valid = validator.validate()
        self.assertTrue(is_valid)
        self.assertTrue(len(validator.get_warnings()) > 0)
    
    def test_unsupported_currency(self):
        """Test that unsupported currency generates warning"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "https://example.com", "role": "attestor", "enabled": True}]}, "compliance": {"supported_currencies": ["USD", "UNKNOWN_CURRENCY"]}}
        validator = ConfigValidator(config)
        is_valid = validator.validate()
        self.assertTrue(is_valid)
        self.assertTrue(len(validator.get_warnings()) > 0)
    
    # Invalid network tests
    def test_invalid_network(self):
        """Test that invalid network fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "invalid-network"}, "attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "https://example.com", "role": "attestor", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("network" in e.lower() for e in validator.get_errors()))
    
    # Invalid Stellar address tests
    def test_invalid_stellar_address_wrong_prefix(self):
        """Test that Stellar address with wrong prefix fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "address": "ABBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB", "endpoint": "https://example.com", "role": "attestor", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("stellar" in e.lower() or "address" in e.lower() for e in validator.get_errors()))
    
    def test_invalid_stellar_address_wrong_length(self):
        """Test that Stellar address with wrong length fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "address": "GBBBBBB", "endpoint": "https://example.com", "role": "attestor", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("address" in e.lower() for e in validator.get_errors()))
    
    # Invalid session config tests
    def test_invalid_session_timeout_too_low(self):
        """Test that session timeout too low fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "https://example.com", "role": "attestor", "enabled": True}]}, "sessions": {"session_timeout_seconds": 30}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("timeout" in e.lower() for e in validator.get_errors()))
    
    def test_invalid_session_timeout_too_high(self):
        """Test that session timeout too high fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "https://example.com", "role": "attestor", "enabled": True}]}, "sessions": {"session_timeout_seconds": 100000}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("timeout" in e.lower() for e in validator.get_errors()))
    
    # Empty registry tests
    def test_empty_attestor_registry(self):
        """Test that empty attestor registry fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": []}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("empty" in e.lower() for e in validator.get_errors()))
    
    def test_no_enabled_attestor(self):
        """Test that no enabled attestor fails validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "test", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "https://example.com", "role": "attestor", "enabled": False}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("enabled" in e.lower() for e in validator.get_errors()))
    
    # Duplicate tests
    def test_duplicate_attestor_names(self):
        """Test that duplicate attestor names fail validation"""
        config = {"contract": {"name": "test", "version": "1.0.0", "network": "stellar-testnet"}, "attestors": {"registry": [{"name": "duplicate", "address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF", "endpoint": "https://example1.com", "role": "attestor", "enabled": True}, {"name": "duplicate", "address": "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB", "endpoint": "https://example2.com", "role": "attestor", "enabled": True}]}}
        validator = ConfigValidator(config)
        self.assertFalse(validator.validate())
        self.assertTrue(any("duplicate" in e.lower() for e in validator.get_errors()))
    
    # Safe failure tests - invalid configs should fail safely
    def test_invalid_config_fails_safely_with_errors(self):
        """Test that invalid config fails safely with clear error messages"""
        config = {}
        validator = ConfigValidator(config)
        is_valid = validator.validate()
        self.assertFalse(is_valid)
        errors = validator.get_errors()
        self.assertTrue(len(errors) > 0)
        for error in errors:
            self.assertTrue(len(error) > 0)
    
    def test_malformed_json_fails_safely(self):
        """Test that malformed JSON fails safely"""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            f.write('{ invalid json }')
            temp_path = f.name
        try:
            is_valid, errors, warnings = validate_config_file(temp_path)
            self.assertFalse(is_valid)
            self.assertTrue(len(errors) > 0)
        finally:
            os.unlink(temp_path)
    
    def test_missing_config_file_fails_safely(self):
        """Test that missing config file fails safely"""
        is_valid, errors, warnings = validate_config_file('/nonexistent/path/config.json')
        self.assertFalse(is_valid)
        self.assertTrue(len(errors) > 0)


def main():
    """Main function to run tests and validate configs"""
    import argparse
    
    parser = argparse.ArgumentParser(description='Validate AnchorKit configurations')
    parser.add_argument('--test', action='store_true', help='Run unit tests')
    parser.add_argument('--validate-all', action='store_true', help='Validate all config files')
    parser.add_argument('config_file', nargs='?', help='Config file to validate')
    
    args = parser.parse_args()
    
    if args.test:
        print("Running configuration validation tests...")
        print("=" * 60)
        unittest.main(argv=[''], exit=False, verbosity=2)
        return
    
    if args.validate_all:
        config_dir = Path('configs')
        if not config_dir.exists():
            print(f"Error: configs directory not found")
            sys.exit(1)
        
        config_files = list(config_dir.glob('*.toml')) + list(config_dir.glob('*.json'))
        
        if not config_files:
            print("No config files found")
            sys.exit(1)
        
        print(f"Validating {len(config_files)} configuration files...")
        print("=" * 60)
        
        errors_found = []
        
        for config_file in config_files:
            is_valid, errors, warnings = validate_config_file(str(config_file))
            
            if warnings:
                print(f"\n⚠️  {config_file.name}:")
                for warning in warnings:
                    print(f"   {warning}")
            
            if is_valid:
                print(f"✅ {config_file.name}")
            else:
                print(f"❌ {config_file.name}:")
                for error in errors:
                    print(f"   • {error}")
                errors_found.append(config_file.name)
        
        print("\n" + "=" * 60)
        if errors_found:
            print(f"❌ Validation failed for: {', '.join(errors_found)}")
            sys.exit(1)
        else:
            print(f"✅ All {len(config_files)} configuration files are valid")
        return
    
    if args.config_file:
        is_valid, errors, warnings = validate_config_file(args.config_file)
        
        print(f"Validating {args.config_file}...")
        print("=" * 60)
        
        if warnings:
            print("\n⚠️  Warnings:")
            for warning in warnings:
                print(f"   {warning}")
        
        if is_valid:
            print("\n✅ Configuration is valid")
            sys.exit(0)
        else:
            print("\n❌ Configuration is invalid:")
            for error in errors:
                print(f"   • {error}")
            sys.exit(1)
    
    parser.print_help()


if __name__ == '__main__':
    main()
