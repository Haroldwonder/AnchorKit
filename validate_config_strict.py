#!/usr/bin/env python3
"""
AnchorKit Configuration Validator
Validates TOML/JSON configs against schema before deployment
Prevents misconfiguration bugs at compile-time
"""

import json
import sys
import toml
from pathlib import Path
from jsonschema import validate, ValidationError, Draft7Validator
import re
from typing import Tuple, List, Dict, Any

class ConfigValidator:
    def __init__(self, schema_path: str):
        with open(schema_path, 'r') as f:
            self.schema = json.load(f)
        self.validator = Draft7Validator(self.schema)
        self.error_count = 0
        self.warning_count = 0
    
    def validate_config(self, config_path: str) -> Tuple[bool, List[str], List[str]]:
        """Validate config file and return (is_valid, errors, warnings)"""
        errors = []
        warnings = []
        
        try:
            # Load config based on extension
            config_file = Path(config_path)
            if config_file.suffix == '.toml':
                with open(config_path, 'r') as f:
                    config = toml.load(f)
            elif config_file.suffix == '.json':
                with open(config_path, 'r') as f:
                    config = json.load(f)
            else:
                return False, [f"Unsupported file format: {config_file.suffix}"], []
            
            # Schema validation
            validation_errors = list(self.validator.iter_errors(config))
            if validation_errors:
                for error in validation_errors:
                    path = '.'.join(str(p) for p in error.path) if error.path else 'root'
                    errors.append(f"[{path}] {error.message}")
                    self.error_count += 1
            
            # Additional business logic validation
            business_errors, business_warnings = self._validate_business_rules(config)
            errors.extend(business_errors)
            warnings.extend(business_warnings)
            
            return len(errors) == 0, errors, warnings
            
        except toml.TomlDecodeError as e:
            return False, [f"TOML parsing error: {str(e)}"], []
        except json.JSONDecodeError as e:
            return False, [f"JSON parsing error: {str(e)}"], []
        except Exception as e:
            return False, [f"Failed to load config: {str(e)}"], []
    
    def _validate_business_rules(self, config: Dict[str, Any]) -> Tuple[List[str], List[str]]:
        """Additional validation beyond schema"""
        errors = []
        warnings = []
        
        # Validate contract config
        if 'contract' in config:
            contract = config['contract']
            
            # Validate name format
            if 'name' in contract:
                if not re.match(r'^[a-z0-9-]+$', contract['name']):
                    errors.append("Contract name must contain only lowercase letters, numbers, and hyphens")
                    self.error_count += 1
            
            # Validate version format
            if 'version' in contract:
                if not re.match(r'^\d+\.\d+\.\d+$', contract['version']):
                    errors.append("Contract version must follow semantic versioning (e.g., 1.0.0)")
                    self.error_count += 1
            
            # Validate network
            if 'network' in contract:
                valid_networks = ['stellar-testnet', 'stellar-mainnet', 'stellar-futurenet']
                if contract['network'] not in valid_networks:
                    errors.append(f"Network must be one of: {', '.join(valid_networks)}")
                    self.error_count += 1
        
        # Validate attestor uniqueness and configuration
        if 'attestors' in config and 'registry' in config['attestors']:
            attestors = config['attestors']['registry']
            
            # Check for duplicate names
            names = [a['name'] for a in attestors]
            duplicates = [name for name in set(names) if names.count(name) > 1]
            if duplicates:
                errors.append(f"Duplicate attestor names found: {', '.join(duplicates)}")
                self.error_count += 1
            
            # Check for duplicate addresses
            addresses = [a['address'] for a in attestors]
            dup_addresses = [addr for addr in set(addresses) if addresses.count(addr) > 1]
            if dup_addresses:
                errors.append(f"Duplicate attestor addresses found: {', '.join(dup_addresses)}")
                self.error_count += 1
            
            # Validate at least one enabled attestor
            enabled = [a for a in attestors if a.get('enabled', False)]
            if not enabled:
                errors.append("At least one attestor must be enabled")
                self.error_count += 1
            
            # Validate each attestor
            for idx, attestor in enumerate(attestors):
                attestor_name = attestor.get('name', f'attestor-{idx}')
                
                # Validate name format
                if 'name' in attestor:
                    if not re.match(r'^[a-z0-9-]+$', attestor['name']):
                        errors.append(f"Attestor '{attestor_name}': name must contain only lowercase letters, numbers, and hyphens")
                        self.error_count += 1
                
                # Validate Stellar address format
                if 'address' in attestor:
                    if not re.match(r'^G[A-Z0-9]{55}$', attestor['address']):
                        errors.append(f"Attestor '{attestor_name}': invalid Stellar address format")
                        self.error_count += 1
                    
                    addr_len = len(attestor['address'])
                    if addr_len < 54 or addr_len > 56:
                        errors.append(f"Attestor '{attestor_name}': address length must be 54-56 characters, got {addr_len}")
                        self.error_count += 1
                
                # Validate endpoint URL
                if 'endpoint' in attestor:
                    if not self._is_valid_url(attestor['endpoint']):
                        errors.append(f"Attestor '{attestor_name}': invalid endpoint URL format")
                        self.error_count += 1
                    
                    if not attestor['endpoint'].startswith('https://'):
                        warnings.append(f"Attestor '{attestor_name}': endpoint should use HTTPS for security")
                        self.warning_count += 1
                
                # Validate role
                if 'role' in attestor:
                    valid_roles = ['kyc-issuer', 'transfer-verifier', 'compliance-approver', 'rate-provider', 'attestor']
                    if attestor['role'] not in valid_roles:
                        errors.append(f"Attestor '{attestor_name}': invalid role. Must be one of: {', '.join(valid_roles)}")
                        self.error_count += 1
        
        # Validate session config consistency
        if 'sessions' in config:
            sessions = config['sessions']
            
            timeout = sessions.get('session_timeout_seconds', 3600)
            if timeout < 60:
                errors.append("Session timeout must be at least 60 seconds")
                self.error_count += 1
            
            if timeout > 86400:
                warnings.append("Session timeout exceeds 24 hours - consider shorter timeouts for security")
                self.warning_count += 1
            
            max_ops = sessions.get('operations_per_session', 1000)
            if max_ops > 5000:
                warnings.append("High operations_per_session may impact performance")
                self.warning_count += 1
        
        return errors, warnings
    
    def _is_valid_url(self, url: str) -> bool:
        """Validate URL format"""
        if not url or len(url) < 8 or len(url) > 256:
            return False
        pattern = r'^https?://[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}(/.*)?$'
        return bool(re.match(pattern, url))

def main():
    if len(sys.argv) < 2:
        print("Usage: python validate_config_strict.py <config_file> [schema_file]")
        sys.exit(1)
    
    config_path = sys.argv[1]
    schema_path = sys.argv[2] if len(sys.argv) > 2 else "config_schema.json"
    
    if not Path(config_path).exists():
        print(f"❌ Config file not found: {config_path}")
        sys.exit(1)
    
    if not Path(schema_path).exists():
        print(f"❌ Schema file not found: {schema_path}")
        sys.exit(1)
    
    validator = ConfigValidator(schema_path)
    is_valid, errors, warnings = validator.validate_config(config_path)
    
    print(f"\n{'='*60}")
    print(f"Configuration Validation: {Path(config_path).name}")
    print(f"{'='*60}\n")
    
    if warnings:
        print("⚠️  Warnings:")
        for warning in warnings:
            print(f"  • {warning}")
        print()
    
    if is_valid:
        print(f"✅ Configuration is valid")
        if validator.warning_count > 0:
            print(f"   ({validator.warning_count} warning(s) - review recommended)")
        sys.exit(0)
    else:
        print(f"❌ Configuration is invalid\n")
        print(f"Errors ({validator.error_count}):")
        for error in errors:
            print(f"  • {error}")
        print()
        sys.exit(1)

if __name__ == "__main__":
    main()
