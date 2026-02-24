#!/usr/bin/env python3
"""
MySQL/MariaDB to PostgreSQL SQL Converter
Converts PEQ database dumps from MySQL format to PostgreSQL format.
"""

import re
import sys
import os

def convert_mysql_to_postgres(input_file, output_file):
    """Convert a MySQL dump file to PostgreSQL format."""
    
    print(f"Converting {input_file} -> {output_file}")
    
    with open(input_file, 'r', encoding='latin-1', errors='replace') as f:
        content = f.read()
    
    # Remove MySQL-specific comments
    content = re.sub(r'/\*!.*?\*/', '', content, flags=re.DOTALL)
    
    # Remove LOCK/UNLOCK TABLES
    content = re.sub(r'LOCK TABLES.*?;', '', content, flags=re.IGNORECASE)
    content = re.sub(r'UNLOCK TABLES\s*;', '', content, flags=re.IGNORECASE)
    
    # Convert backticks to double quotes for identifiers
    content = content.replace('`', '"')
    
    # Convert AUTO_INCREMENT to SERIAL
    content = re.sub(r'int\(\d+\)\s+(unsigned\s+)?NOT NULL AUTO_INCREMENT', 'SERIAL', content, flags=re.IGNORECASE)
    content = re.sub(r'int\(\d+\)\s+AUTO_INCREMENT', 'SERIAL', content, flags=re.IGNORECASE)
    content = re.sub(r'bigint\(\d+\)\s+(unsigned\s+)?NOT NULL AUTO_INCREMENT', 'BIGSERIAL', content, flags=re.IGNORECASE)
    
    # Convert MySQL data types to PostgreSQL
    # Crucial: MySQL 'int unsigned' (4e9) fits in Postgres 'bigint' (9e18), but not 'integer' (2e9)
    # Safer to map ALL int -> BIGINT to avoid overflow during import
    content = re.sub(r'\bint\(\d+\)\s+unsigned', 'BIGINT', content, flags=re.IGNORECASE)
    content = re.sub(r'\bint\s+unsigned', 'BIGINT', content, flags=re.IGNORECASE)
    content = re.sub(r'\bint\(\d+\)', 'BIGINT', content, flags=re.IGNORECASE)
    content = re.sub(r'\bint\b', 'BIGINT', content, flags=re.IGNORECASE)
    
    content = re.sub(r'\bbigint\(\d+\)\s+unsigned', 'BIGINT', content, flags=re.IGNORECASE)
    content = re.sub(r'\bbigint\(\d+\)', 'BIGINT', content, flags=re.IGNORECASE)
    
    content = re.sub(r'\bsmallint\(\d+\)\s+unsigned', 'INTEGER', content, flags=re.IGNORECASE)
    content = re.sub(r'\bsmallint\(\d+\)', 'SMALLINT', content, flags=re.IGNORECASE)
    
    content = re.sub(r'\btinyint\(\d+\)\s+unsigned', 'SMALLINT', content, flags=re.IGNORECASE)
    content = re.sub(r'\btinyint\(\d+\)', 'SMALLINT', content, flags=re.IGNORECASE)
    content = re.sub(r'\btinyint', 'SMALLINT', content, flags=re.IGNORECASE)
    
    content = re.sub(r'\bmediumint\(\d+\)\s+unsigned', 'INTEGER', content, flags=re.IGNORECASE)
    content = re.sub(r'\bmediumint\(\d+\)', 'INTEGER', content, flags=re.IGNORECASE)
    
    content = re.sub(r'\bfloat\(\d+,\d+\)', 'REAL', content, flags=re.IGNORECASE)
    content = re.sub(r'\bfloat\s+unsigned', 'REAL', content, flags=re.IGNORECASE)
    content = re.sub(r'\bdouble\(\d+,\d+\)', 'DOUBLE PRECISION', content, flags=re.IGNORECASE)
    content = re.sub(r'\bdouble\b', 'DOUBLE PRECISION', content, flags=re.IGNORECASE)
    content = re.sub(r'\bdatetime\b', 'TIMESTAMP', content, flags=re.IGNORECASE)
    
    # Text/Blob types
    content = re.sub(r'\blongtext\b', 'TEXT', content, flags=re.IGNORECASE)
    content = re.sub(r'\bmediumtext\b', 'TEXT', content, flags=re.IGNORECASE)
    content = re.sub(r'\btinytext\b', 'TEXT', content, flags=re.IGNORECASE)
    content = re.sub(r'\blongblob\b', 'BYTEA', content, flags=re.IGNORECASE)
    content = re.sub(r'\bmediumblob\b', 'BYTEA', content, flags=re.IGNORECASE)
    content = re.sub(r'\bblob\b', 'BYTEA', content, flags=re.IGNORECASE)
    content = re.sub(r'\btinyblob\b', 'BYTEA', content, flags=re.IGNORECASE)
    
    # Map varchar to TEXT to avoid length issues during import
    content = re.sub(r'\bvarchar\(\d+\)', 'TEXT', content, flags=re.IGNORECASE)
    
    # Remove ZEROFILL (MySQL formatting option, not valid in PG)
    content = re.sub(r'\s+ZEROFILL', '', content, flags=re.IGNORECASE)
    
    # Convert current_timestamp() to CURRENT_TIMESTAMP
    content = re.sub(r'current_timestamp\(\)', 'CURRENT_TIMESTAMP', content, flags=re.IGNORECASE)
    content = re.sub(r'now\(\)', 'CURRENT_TIMESTAMP', content, flags=re.IGNORECASE)
    
    # Remove USING BTREE/HASH from PRIMARY KEY and indexes
    content = re.sub(r'\)\s*USING BTREE', ')', content, flags=re.IGNORECASE)
    content = re.sub(r'\)\s*USING HASH', ')', content, flags=re.IGNORECASE)
    content = re.sub(r'USING BTREE', '', content, flags=re.IGNORECASE)
    content = re.sub(r'USING HASH', '', content, flags=re.IGNORECASE)
    
    # Remove ON UPDATE CURRENT_TIMESTAMP (PostgreSQL uses triggers for this)
    content = re.sub(r'ON UPDATE CURRENT_TIMESTAMP', '', content, flags=re.IGNORECASE)
    
    # Remove ENGINE=... and CHARSET=...
    content = re.sub(r'\)\s*ENGINE=\w+(\s+DEFAULT)?\s*CHARSET=\w+(\s+COLLATE=\w+)?;', ');', content, flags=re.IGNORECASE)
    content = re.sub(r'\)\s*ENGINE=\w+;', ');', content, flags=re.IGNORECASE)
    content = re.sub(r'ENGINE=\w+', '', content, flags=re.IGNORECASE)
    content = re.sub(r'DEFAULT CHARSET=\w+', '', content, flags=re.IGNORECASE)
    content = re.sub(r'CHARSET=\w+', '', content, flags=re.IGNORECASE)
    content = re.sub(r'COLLATE=\w+', '', content, flags=re.IGNORECASE)
    content = re.sub(r'COLLATE \w+', '', content, flags=re.IGNORECASE)
    content = re.sub(r'CHARACTER SET \w+', '', content, flags=re.IGNORECASE)
    
    # Remove ROW_FORMAT=... (MySQL storage hint)
    content = re.sub(r'ROW_FORMAT=\w+', '', content, flags=re.IGNORECASE)
    
    # Remove PACK_KEYS=...
    content = re.sub(r'PACK_KEYS=\d+', '', content, flags=re.IGNORECASE)
    
    # Fix "DOUBLE PRECISION NOT NULL" after double->DOUBLE PRECISION conversion
    # (already fine, but ensure no issues with subsequent tokens)
    content = re.sub(r'DOUBLE PRECISION PRECISION', 'DOUBLE PRECISION', content, flags=re.IGNORECASE)


    
    # Remove AUTO_INCREMENT=nnn
    content = re.sub(r'AUTO_INCREMENT=\d+', '', content, flags=re.IGNORECASE)
    
    # Convert MySQL KEY to PostgreSQL index (after table creation)
    # For now, just remove inline KEY definitions (indexes need separate CREATE INDEX)
    # Handle multi-line patterns with DOTALL
    content = re.sub(r',\s*KEY\s+"[^"]+"\s*\([^)]+\)', '', content, flags=re.IGNORECASE | re.DOTALL)
    content = re.sub(r',\s*UNIQUE\s+KEY\s+"[^"]+"\s*\([^)]+\)', '', content, flags=re.IGNORECASE | re.DOTALL)
    content = re.sub(r',\s*FULLTEXT\s+KEY\s+"[^"]+"\s*\([^)]+\)', '', content, flags=re.IGNORECASE | re.DOTALL)
    # Also handle UNIQUE KEY without name
    content = re.sub(r',\s*UNIQUE\s+KEY\s*\([^)]+\)', '', content, flags=re.IGNORECASE | re.DOTALL)
    content = re.sub(r',\s*KEY\s*\([^)]+\)', '', content, flags=re.IGNORECASE | re.DOTALL)
    
    # Garbage collection for stray UN/UNLOCK artifacts
    content = re.sub(r'^\s*UN\s*$', '', content, flags=re.MULTILINE)
    
    # Convert MySQL escaping in VALUES
    content = content.replace("\\'", "''")
    
    # Remove trailing commas before closing parenthesis in CREATE TABLE
    content = re.sub(r',\s*\n\s*\)', '\n)', content)
    
    # Write output
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write("-- Converted from MySQL to PostgreSQL\n")
        f.write("SET client_encoding = 'UTF8';\n\n")
        f.write(content)
    
    print(f"Done! Output: {output_file}")

def main():
    if len(sys.argv) < 3:
        print("Usage: python mysql_to_postgres.py <input.sql> <output.sql>")
        sys.exit(1)
    
    input_file = sys.argv[1]
    output_file = sys.argv[2]
    
    if not os.path.exists(input_file):
        print(f"Error: Input file not found: {input_file}")
        sys.exit(1)
    
    convert_mysql_to_postgres(input_file, output_file)

if __name__ == "__main__":
    main()
