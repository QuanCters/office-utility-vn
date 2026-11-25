#!/usr/bin/env python3
"""
Forensic XLSX Reconciliation and Data Integrity Auditor
Exhaustively inspects test fixtures and saved artifacts for Stage 4C-0C Acceptance Gate.
"""
import os
import sys
import hashlib
import zipfile
import xml.etree.ElementTree as ET
import openpyxl

def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(8192):
            h.update(chunk)
    return h.hexdigest()

def inspect_openpyxl(path):
    wb = openpyxl.load_workbook(path, data_only=False)
    results = {
        "sheets": wb.sheetnames,
        "cells": {}
    }
    for sheet_name in wb.sheetnames:
        ws = wb[sheet_name]
        results["cells"][sheet_name] = {}
        for r in range(1, 10):
            for c in range(1, 10):
                cell = ws.cell(row=r, column=c)
                if cell.value is not None:
                    coord = f"{chr(64+c)}{r}"
                    results["cells"][sheet_name][coord] = {
                        "value": cell.value,
                        "data_type": cell.data_type
                    }
    return results

def inspect_raw_xml(path):
    xml_data = {}
    with zipfile.ZipFile(path, 'r') as z:
        for name in z.namelist():
            if name.endswith('.xml') or name.endswith('.rels'):
                xml_data[name] = z.read(name).decode('utf-8', errors='ignore')
    return xml_data

def main():
    print("=== FORENSIC XLSX RECONCILIATION AUDIT ===")
    base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    fixture_path = os.path.join(base_dir, "fixtures", "test_uat_sample.xlsx")
    saved_path = os.path.join(base_dir, "artifacts", "tc04_saved_fixture.xlsx")

    print(f"\n1. Fixture: {fixture_path}")
    print(f"   SHA-256: {sha256_file(fixture_path)}")
    fx_pyxl = inspect_openpyxl(fixture_path)
    print(f"   Sheets: {fx_pyxl['sheets']}")
    for s, cells in fx_pyxl["cells"].items():
        print(f"   Sheet '{s}' Cells:")
        for coord, val in cells.items():
            print(f"     {coord} = {val['value']!r} ({val['data_type']})")

    print(f"\n2. Saved Artifact: {saved_path}")
    print(f"   SHA-256: {sha256_file(saved_path)}")
    sv_pyxl = inspect_openpyxl(saved_path)
    print(f"   Sheets: {sv_pyxl['sheets']}")
    for s, cells in sv_pyxl["cells"].items():
        print(f"   Sheet '{s}' Cells:")
        for coord, val in cells.items():
            print(f"     {coord} = {val['value']!r} ({val['data_type']})")

    print("\n3. Raw OpenXML Structural Discrepancy Analysis:")
    fx_xml = inspect_raw_xml(fixture_path)
    sv_xml = inspect_raw_xml(saved_path)

    # Check shared strings
    print("   [SharedStrings Comparison]")
    print(f"   Fixture has sharedStrings: {'xl/sharedStrings.xml' in fx_xml}")
    if 'xl/sharedStrings.xml' in fx_xml:
        print(f"     Fixture XML snippet: {fx_xml['xl/sharedStrings.xml'][:200]}")
    print(f"   Saved artifact has sharedStrings: {'xl/sharedStrings.xml' in sv_xml}")
    if 'xl/sharedStrings.xml' in sv_xml:
        print(f"     Saved artifact XML snippet: {sv_xml['xl/sharedStrings.xml'][:200]}")

    # Check worksheet xml
    print("   [Worksheet XML Comparison]")
    for k in fx_xml:
        if 'worksheets' in k:
            print(f"     Fixture {k} snippet: {fx_xml[k][:300]}")
    for k in sv_xml:
        if 'worksheets' in k:
            print(f"     Saved artifact {k} snippet: {sv_xml[k][:300]}")

if __name__ == "__main__":
    main()
