#!/usr/bin/env python3
"""
Script to add PaykitDemoTests target to Xcode project.
This script modifies the project.pbxproj file to add a test target.

NOTE: This script was used during initial test infrastructure setup.
The test target is now configured in the project. This script is kept
for reference in case the test target needs to be recreated.
"""

import re
import uuid
import sys

def generate_id():
    """Generate a 24-character hex ID for Xcode project objects"""
    return uuid.uuid4().hex[:24].upper()

def add_test_target(project_path):
    """Add PaykitDemoTests target to the Xcode project"""
    
    with open(project_path, 'r') as f:
        content = f.read()
    
    # Generate IDs for new objects
    test_target_id = generate_id()
    test_product_id = generate_id()
    test_sources_phase_id = generate_id()
    test_frameworks_phase_id = generate_id()
    test_build_config_list_id = generate_id()
    test_debug_config_id = generate_id()
    test_release_config_id = generate_id()
    test_group_id = generate_id()
    
    # Find the main target ID
    main_target_match = re.search(r'5224F5F42EED89A600A4DEB4 /\* PaykitDemo \*/', content)
    if not main_target_match:
        print("Error: Could not find main target")
        return False
    
    # Find where to insert new sections
    # Add test target file reference
    file_ref_section = re.search(r'(/\* Begin PBXFileReference section \*/)', content)
    if file_ref_section:
        insert_pos = file_ref_section.end()
        test_file_ref = f'\t\t{test_product_id} /* PaykitDemoTests.xctest */ = {{isa = PBXFileReference; explicitFileType = wrapper.cfbundle; includeInIndex = 0; path = PaykitDemoTests.xctest; sourceTree = BUILT_PRODUCTS_DIR; }};\n'
        content = content[:insert_pos] + test_file_ref + content[insert_pos:]
    
    # Add test group
    group_section = re.search(r'(/\* Begin PBXGroup section \*/)', content)
    if group_section:
        # Find the main group children
        main_group_match = re.search(r'(5224F5EC2EED89A600A4DEB4 = \{[^}]+children = \()', content)
        if main_group_match:
            insert_pos = main_group_match.end()
            test_group_ref = f'\t\t\t\t{test_group_id} /* PaykitDemoTests */,\n'
            content = content[:insert_pos] + test_group_ref + content[insert_pos:]
        
        # Add the test group definition before End PBXGroup section
        group_end = re.search(r'(/\* End PBXGroup section \*/)', content)
        if group_end:
            insert_pos = group_end.start()
            test_group_def = f'''\t\t{test_group_id} /* PaykitDemoTests */ = {{
\t\t\tisa = PBXGroup;
\t\t\tchildren = (
\t\t\t);
\t\t\tpath = PaykitDemoTests;
\t\t\tsourceTree = "<group>";
\t\t}};
'''
            content = content[:insert_pos] + test_group_def + content[insert_pos:]
    
    # Add test target to Products group
    products_group_match = re.search(r'(5224F5F62EED89A600A4DEB4 /\* Products \*/ = \{[^}]+children = \()', content)
    if products_group_match:
        insert_pos = products_group_match.end()
        test_product_ref = f'\t\t\t\t{test_product_id} /* PaykitDemoTests.xctest */,\n'
        content = content[:insert_pos] + test_product_ref + content[insert_pos:]
    
    # Add build phases for test target
    sources_phase_end = re.search(r'(/\* End PBXSourcesBuildPhase section \*/)', content)
    if sources_phase_end:
        insert_pos = sources_phase_end.start()
        test_sources_phase = f'''\t\t{test_sources_phase_id} /* Sources */ = {{
\t\t\tisa = PBXSourcesBuildPhase;
\t\t\tbuildActionMask = 2147483647;
\t\t\tfiles = (
\t\t\t);
\t\t\trunOnlyForDeploymentPostprocessing = 0;
\t\t}};
'''
        content = content[:insert_pos] + test_sources_phase + content[insert_pos:]
    
    frameworks_phase_end = re.search(r'(/\* End PBXFrameworksBuildPhase section \*/)', content)
    if frameworks_phase_end:
        insert_pos = frameworks_phase_end.start()
        test_frameworks_phase = f'''\t\t{test_frameworks_phase_id} /* Frameworks */ = {{
\t\t\tisa = PBXFrameworksBuildPhase;
\t\t\tbuildActionMask = 2147483647;
\t\t\tfiles = (
\t\t\t);
\t\t\trunOnlyForDeploymentPostprocessing = 0;
\t\t}};
'''
        content = content[:insert_pos] + test_frameworks_phase + content[insert_pos:]
    
    # Add test target
    native_target_end = re.search(r'(/\* End PBXNativeTarget section \*/)', content)
    if native_target_end:
        insert_pos = native_target_end.start()
        test_target = f'''\t\t{test_target_id} /* PaykitDemoTests */ = {{
\t\t\tisa = PBXNativeTarget;
\t\t\tbuildConfigurationList = {test_build_config_list_id} /* Build configuration list for PBXNativeTarget "PaykitDemoTests" */;
\t\t\tbuildPhases = (
\t\t\t\t{test_sources_phase_id} /* Sources */,
\t\t\t\t{test_frameworks_phase_id} /* Frameworks */,
\t\t\t);
\t\t\tbuildRules = (
\t\t\t);
\t\t\tdependencies = (
\t\t\t\t{generate_id()} /* PBXTargetDependency */,
\t\t\t);
\t\t\tname = PaykitDemoTests;
\t\t\tpackageProductDependencies = (
\t\t\t);
\t\t\tproductName = PaykitDemoTests;
\t\t\tproductReference = {test_product_id} /* PaykitDemoTests.xctest */;
\t\t\tproductType = "com.apple.product-type.bundle.unit-test";
\t\t}};
'''
        content = content[:insert_pos] + test_target + content[insert_pos:]
    
    # Add test target to project targets list
    targets_match = re.search(r'(targets = \()', content)
    if targets_match:
        insert_pos = targets_match.end()
        test_target_ref = f'\t\t\t\t{test_target_id} /* PaykitDemoTests */,\n'
        content = content[:insert_pos] + test_target_ref + content[insert_pos:]
    
    # Add build configurations for test target
    config_list_end = re.search(r'(/\* End XCConfigurationList section \*/)', content)
    if config_list_end:
        insert_pos = config_list_end.start()
        test_debug_config = f'''\t\t{test_debug_config_id} /* Debug */ = {{
\t\t\tisa = XCBuildConfiguration;
\t\t\tbuildSettings = {{
\t\t\t\tBUNDLE_LOADER = "$(TEST_HOST)";
\t\t\t\tCODE_SIGN_STYLE = Automatic;
\t\t\t\tCURRENT_PROJECT_VERSION = 1;
\t\t\t\tGENERATE_INFOPLIST_FILE = YES;
\t\t\t\tINFOPLIST_KEY_UIApplicationSupportsIndirectInputEvents = YES;
\t\t\t\tIPHONEOS_DEPLOYMENT_TARGET = 16.6;
\t\t\t\tLD_RUNPATH_SEARCH_PATHS = (
\t\t\t\t\t"$(inherited)",
\t\t\t\t\t"@executable_path/Frameworks",
\t\t\t\t\t"@loader_path/Frameworks",
\t\t\t\t);
\t\t\t\tMARKETING_VERSION = 1.0;
\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = synonym.PaykitDemoTests;
\t\t\t\tPRODUCT_NAME = "$(TARGET_NAME)";
\t\t\t\tSWIFT_EMIT_LOC_STRINGS = NO;
\t\t\t\tSWIFT_VERSION = 5.0;
\t\t\t\tTEST_HOST = "$(BUILT_PRODUCTS_DIR)/PaykitDemo.app/$(BUNDLE_EXECUTABLE_FOLDER_PATH)/PaykitDemo";
\t\t\t}};
\t\t\tname = Debug;
\t\t}};
\t\t{test_release_config_id} /* Release */ = {{
\t\t\tisa = XCBuildConfiguration;
\t\t\tbuildSettings = {{
\t\t\t\tBUNDLE_LOADER = "$(TEST_HOST)";
\t\t\t\tCODE_SIGN_STYLE = Automatic;
\t\t\t\tCURRENT_PROJECT_VERSION = 1;
\t\t\t\tGENERATE_INFOPLIST_FILE = YES;
\t\t\t\tINFOPLIST_KEY_UIApplicationSupportsIndirectInputEvents = YES;
\t\t\t\tIPHONEOS_DEPLOYMENT_TARGET = 16.6;
\t\t\t\tLD_RUNPATH_SEARCH_PATHS = (
\t\t\t\t\t"$(inherited)",
\t\t\t\t\t"@executable_path/Frameworks",
\t\t\t\t"@loader_path/Frameworks",
\t\t\t);
\t\t\t\tMARKETING_VERSION = 1.0;
\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = synonym.PaykitDemoTests;
\t\t\t\tPRODUCT_NAME = "$(TARGET_NAME)";
\t\t\t\tSWIFT_EMIT_LOC_STRINGS = NO;
\t\t\t\tSWIFT_VERSION = 5.0;
\t\t\t\tTEST_HOST = "$(BUILT_PRODUCTS_DIR)/PaykitDemo.app/$(BUNDLE_EXECUTABLE_FOLDER_PATH)/PaykitDemo";
\t\t\t}};
\t\t\tname = Release;
\t\t}};
\t\t{test_build_config_list_id} /* Build configuration list for PBXNativeTarget "PaykitDemoTests" */ = {{
\t\t\tisa = XCConfigurationList;
\t\t\tbuildConfigurations = (
\t\t\t\t{test_debug_config_id} /* Debug */,
\t\t\t\t{test_release_config_id} /* Release */,
\t\t\t);
\t\t\tdefaultConfigurationIsVisible = 0;
\t\t\tdefaultConfigurationName = Release;
\t\t}};
'''
        content = content[:insert_pos] + test_debug_config + content[insert_pos:]
    
    # Update target attributes
    target_attrs_match = re.search(r'(TargetAttributes = \{)', content)
    if target_attrs_match:
        insert_pos = target_attrs_match.end()
        test_target_attrs = f'''\t\t\t\t{test_target_id} = {{
\t\t\t\t\tCreatedOnToolsVersion = 26.1.1;
\t\t\t\t\tTestTargetID = 5224F5F42EED89A600A4DEB4;
\t\t\t\t}};
'''
        content = content[:insert_pos] + test_target_attrs + content[insert_pos:]
    
    # Write back
    with open(project_path, 'w') as f:
        f.write(content)
    
    print(f"Added PaykitDemoTests target to project")
    print(f"Test target ID: {test_target_id}")
    return True

if __name__ == '__main__':
    project_path = 'PaykitDemo.xcodeproj/project.pbxproj'
    if add_test_target(project_path):
        print("Successfully added test target")
        sys.exit(0)
    else:
        print("Failed to add test target")
        sys.exit(1)
