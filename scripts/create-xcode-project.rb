#!/usr/bin/env ruby
#
# create-xcode-project.rb
# Creates a properly configured Xcode project for AirGapSync
#

require 'fileutils'
require 'securerandom'

PROJECT_DIR = "/Users/parobek/Code/AirGapSync"
PROJECT_NAME = "AirGapSync"
BUNDLE_ID = "com.airgapsync.app"

# Generate UUIDs for Xcode objects
def uuid
  SecureRandom.uuid.gsub('-', '').upcase[0..23]
end

# Create the project structure
puts "Creating Xcode project for #{PROJECT_NAME}..."

# Backup existing project
if File.directory?("#{PROJECT_DIR}/#{PROJECT_NAME}.xcodeproj")
  backup_name = "#{PROJECT_NAME}.xcodeproj.backup.#{Time.now.strftime('%Y%m%d_%H%M%S')}"
  puts "Backing up existing project to #{backup_name}..."
  FileUtils.mv("#{PROJECT_DIR}/#{PROJECT_NAME}.xcodeproj", "#{PROJECT_DIR}/#{backup_name}")
end

# Create project directory
FileUtils.mkdir_p("#{PROJECT_DIR}/#{PROJECT_NAME}.xcodeproj")

# Generate UUIDs for all objects
app_target_uuid = uuid
app_product_uuid = uuid
main_group_uuid = uuid
products_group_uuid = uuid
sources_group_uuid = uuid
frameworks_phase_uuid = uuid
sources_phase_uuid = uuid
resources_phase_uuid = uuid
build_config_debug_uuid = uuid
build_config_release_uuid = uuid
config_list_target_uuid = uuid
config_list_project_uuid = uuid
project_uuid = uuid
script_phase_uuid = uuid

# Source files
menubar_uuid = uuid
syncmanager_uuid = uuid
ffibridge_uuid = uuid
menubar_build_uuid = uuid
syncmanager_build_uuid = uuid
ffibridge_build_uuid = uuid

# Create pbxproj content
pbxproj = <<~PBXPROJ
// !$*UTF8*$!
{
\tarchiveVersion = 1;
\tclasses = {
\t};
\tobjectVersion = 54;
\tobjects = {

/* Begin PBXBuildFile section */
\t\t#{menubar_build_uuid} /* MenuBarApp.swift in Sources */ = {isa = PBXBuildFile; fileRef = #{menubar_uuid} /* MenuBarApp.swift */; };
\t\t#{syncmanager_build_uuid} /* SyncManager.swift in Sources */ = {isa = PBXBuildFile; fileRef = #{syncmanager_uuid} /* SyncManager.swift */; };
\t\t#{ffibridge_build_uuid} /* FFIBridge.swift in Sources */ = {isa = PBXBuildFile; fileRef = #{ffibridge_uuid} /* FFIBridge.swift */; };
/* End PBXBuildFile section */

/* Begin PBXFileReference section */
\t\t#{app_product_uuid} /* #{PROJECT_NAME}.app */ = {isa = PBXFileReference; explicitFileType = wrapper.application; includeInIndex = 0; path = #{PROJECT_NAME}.app; sourceTree = BUILT_PRODUCTS_DIR; };
\t\t#{menubar_uuid} /* MenuBarApp.swift */ = {isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = MenuBarApp.swift; sourceTree = "<group>"; };
\t\t#{syncmanager_uuid} /* SyncManager.swift */ = {isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = SyncManager.swift; sourceTree = "<group>"; };
\t\t#{ffibridge_uuid} /* FFIBridge.swift */ = {isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = FFIBridge.swift; sourceTree = "<group>"; };
/* End PBXFileReference section */

/* Begin PBXFrameworksBuildPhase section */
\t\t#{frameworks_phase_uuid} /* Frameworks */ = {
\t\t\tisa = PBXFrameworksBuildPhase;
\t\t\tbuildActionMask = 2147483647;
\t\t\tfiles = (
\t\t\t);
\t\t\trunOnlyForDeploymentPostprocessing = 0;
\t\t};
/* End PBXFrameworksBuildPhase section */

/* Begin PBXGroup section */
\t\t#{main_group_uuid} = {
\t\t\tisa = PBXGroup;
\t\t\tchildren = (
\t\t\t\t#{sources_group_uuid} /* AirGapSync */,
\t\t\t\t#{products_group_uuid} /* Products */,
\t\t\t);
\t\t\tsourceTree = "<group>";
\t\t};
\t\t#{sources_group_uuid} /* AirGapSync */ = {
\t\t\tisa = PBXGroup;
\t\t\tchildren = (
\t\t\t\t#{menubar_uuid} /* MenuBarApp.swift */,
\t\t\t\t#{syncmanager_uuid} /* SyncManager.swift */,
\t\t\t\t#{ffibridge_uuid} /* FFIBridge.swift */,
\t\t\t);
\t\t\tpath = AirGapSync/AirGapSync;
\t\t\tsourceTree = "<group>";
\t\t};
\t\t#{products_group_uuid} /* Products */ = {
\t\t\tisa = PBXGroup;
\t\t\tchildren = (
\t\t\t\t#{app_product_uuid} /* #{PROJECT_NAME}.app */,
\t\t\t);
\t\t\tname = Products;
\t\t\tsourceTree = "<group>";
\t\t};
/* End PBXGroup section */

/* Begin PBXNativeTarget section */
\t\t#{app_target_uuid} /* #{PROJECT_NAME} */ = {
\t\t\tisa = PBXNativeTarget;
\t\t\tbuildConfigurationList = #{config_list_target_uuid} /* Build configuration list for PBXNativeTarget "#{PROJECT_NAME}" */;
\t\t\tbuildPhases = (
\t\t\t\t#{script_phase_uuid} /* Build Rust Library */,
\t\t\t\t#{sources_phase_uuid} /* Sources */,
\t\t\t\t#{frameworks_phase_uuid} /* Frameworks */,
\t\t\t\t#{resources_phase_uuid} /* Resources */,
\t\t\t);
\t\t\tbuildRules = (
\t\t\t);
\t\t\tdependencies = (
\t\t\t);
\t\t\tname = #{PROJECT_NAME};
\t\t\tproductName = #{PROJECT_NAME};
\t\t\tproductReference = #{app_product_uuid} /* #{PROJECT_NAME}.app */;
\t\t\tproductType = "com.apple.product-type.application";
\t\t};
/* End PBXNativeTarget section */

/* Begin PBXProject section */
\t\t#{project_uuid} /* Project object */ = {
\t\t\tisa = PBXProject;
\t\t\tattributes = {
\t\t\t\tLastSwiftUpdateCheck = 1500;
\t\t\t\tLastUpgradeCheck = 1500;
\t\t\t\tTargetAttributes = {
\t\t\t\t\t#{app_target_uuid} = {
\t\t\t\t\t\tCreatedOnToolsVersion = 15.0;
\t\t\t\t\t};
\t\t\t\t};
\t\t\t};
\t\t\tbuildConfigurationList = #{config_list_project_uuid} /* Build configuration list for PBXProject "#{PROJECT_NAME}" */;
\t\t\tcompatibilityVersion = "Xcode 12.0";
\t\t\tdevelopmentRegion = en;
\t\t\thasScannedForEncodings = 0;
\t\t\tknownRegions = (
\t\t\t\ten,
\t\t\t\tBase,
\t\t\t);
\t\t\tmainGroup = #{main_group_uuid};
\t\t\tproductRefGroup = #{products_group_uuid} /* Products */;
\t\t\tprojectDirPath = "";
\t\t\tprojectRoot = "";
\t\t\ttargets = (
\t\t\t\t#{app_target_uuid} /* #{PROJECT_NAME} */,
\t\t\t);
\t\t};
/* End PBXProject section */

/* Begin PBXResourcesBuildPhase section */
\t\t#{resources_phase_uuid} /* Resources */ = {
\t\t\tisa = PBXResourcesBuildPhase;
\t\t\tbuildActionMask = 2147483647;
\t\t\tfiles = (
\t\t\t);
\t\t\trunOnlyForDeploymentPostprocessing = 0;
\t\t};
/* End PBXResourcesBuildPhase section */

/* Begin PBXShellScriptBuildPhase section */
\t\t#{script_phase_uuid} /* Build Rust Library */ = {
\t\t\tisa = PBXShellScriptBuildPhase;
\t\t\tbuildActionMask = 2147483647;
\t\t\tfiles = (
\t\t\t);
\t\t\tinputFileListPaths = (
\t\t\t);
\t\t\tinputPaths = (
\t\t\t);
\t\t\tname = "Build Rust Library";
\t\t\toutputFileListPaths = (
\t\t\t);
\t\t\toutputPaths = (
\t\t\t);
\t\t\trunOnlyForDeploymentPostprocessing = 0;
\t\t\tshellPath = /bin/sh;
\t\t\tshellScript = "cd \\"${SRCROOT}\\"\\necho \\"Building Rust library...\\"\\ncargo build --lib --release\\nmkdir -p lib\\ncp target/release/libairgap_sync.a lib/\\ncp target/airgapsync.h lib/\\necho \\"Rust library built successfully\\"\\n";
\t\t};
/* End PBXShellScriptBuildPhase section */

/* Begin PBXSourcesBuildPhase section */
\t\t#{sources_phase_uuid} /* Sources */ = {
\t\t\tisa = PBXSourcesBuildPhase;
\t\t\tbuildActionMask = 2147483647;
\t\t\tfiles = (
\t\t\t\t#{menubar_build_uuid} /* MenuBarApp.swift in Sources */,
\t\t\t\t#{syncmanager_build_uuid} /* SyncManager.swift in Sources */,
\t\t\t\t#{ffibridge_build_uuid} /* FFIBridge.swift in Sources */,
\t\t\t);
\t\t\trunOnlyForDeploymentPostprocessing = 0;
\t\t};
/* End PBXSourcesBuildPhase section */

/* Begin XCBuildConfiguration section */
\t\t#{build_config_debug_uuid} /* Debug */ = {
\t\t\tisa = XCBuildConfiguration;
\t\t\tbuildSettings = {
\t\t\t\tALWAYS_SEARCH_USER_PATHS = NO;
\t\t\t\tCLANG_ANALYZER_NONNULL = YES;
\t\t\t\tCLANG_CXX_LANGUAGE_STANDARD = "gnu++17";
\t\t\t\tCLANG_WARN_BLOCK_CAPTURE_AUTORELEASING = YES;
\t\t\t\tCLANG_WARN_DOCUMENTATION_COMMENTS = YES;
\t\t\t\tCOPY_PHASE_STRIP = NO;
\t\t\t\tDEBUG_INFORMATION_FORMAT = dwarf;
\t\t\t\tENABLE_STRICT_OBJC_MSGSEND = YES;
\t\t\t\tENABLE_TESTABILITY = YES;
\t\t\t\tGCC_NO_COMMON_BLOCKS = YES;
\t\t\t\tGCC_OPTIMIZATION_LEVEL = 0;
\t\t\t\tGCC_WARN_ABOUT_RETURN_TYPE = YES_ERROR;
\t\t\t\tGCC_WARN_UNINITIALIZED_AUTOS = YES_AGGRESSIVE;
\t\t\t\tMACOSX_DEPLOYMENT_TARGET = 13.0;
\t\t\t\tONLY_ACTIVE_ARCH = YES;
\t\t\t\tSDKROOT = macosx;
\t\t\t\tSWIFT_ACTIVE_COMPILATION_CONDITIONS = DEBUG;
\t\t\t\tSWIFT_OPTIMIZATION_LEVEL = "-Onone";
\t\t\t};
\t\t\tname = Debug;
\t\t};
\t\t#{build_config_release_uuid} /* Release */ = {
\t\t\tisa = XCBuildConfiguration;
\t\t\tbuildSettings = {
\t\t\t\tALWAYS_SEARCH_USER_PATHS = NO;
\t\t\t\tCLANG_ANALYZER_NONNULL = YES;
\t\t\t\tCLANG_CXX_LANGUAGE_STANDARD = "gnu++17";
\t\t\t\tCLANG_WARN_BLOCK_CAPTURE_AUTORELEASING = YES;
\t\t\t\tCLANG_WARN_DOCUMENTATION_COMMENTS = YES;
\t\t\t\tCOPY_PHASE_STRIP = NO;
\t\t\t\tDEBUG_INFORMATION_FORMAT = "dwarf-with-dsym";
\t\t\t\tENABLE_NS_ASSERTIONS = NO;
\t\t\t\tENABLE_STRICT_OBJC_MSGSEND = YES;
\t\t\t\tGCC_NO_COMMON_BLOCKS = YES;
\t\t\t\tGCC_WARN_ABOUT_RETURN_TYPE = YES_ERROR;
\t\t\t\tGCC_WARN_UNINITIALIZED_AUTOS = YES_AGGRESSIVE;
\t\t\t\tMACOSX_DEPLOYMENT_TARGET = 13.0;
\t\t\t\tSDKROOT = macosx;
\t\t\t\tSWIFT_COMPILATION_MODE = wholemodule;
\t\t\t\tSWIFT_OPTIMIZATION_LEVEL = "-O";
\t\t\t};
\t\t\tname = Release;
\t\t};
\t\t#{uuid} /* Debug */ = {
\t\t\tisa = XCBuildConfiguration;
\t\t\tbuildSettings = {
\t\t\t\tASSETCATALOG_COMPILER_APPICON_NAME = AppIcon;
\t\t\t\tCODE_SIGN_STYLE = Automatic;
\t\t\t\tCOMBINE_HIDPI_IMAGES = YES;
\t\t\t\tCURRENT_PROJECT_VERSION = 1;
\t\t\t\tDEVELOPMENT_TEAM = "";
\t\t\t\tENABLE_HARDENED_RUNTIME = YES;
\t\t\t\tENABLE_PREVIEWS = YES;
\t\t\t\tGENERATE_INFOPLIST_FILE = YES;
\t\t\t\tHEADER_SEARCH_PATHS = (
\t\t\t\t\t"\$(SRCROOT)/lib",
\t\t\t\t\t"\$(SRCROOT)/target",
\t\t\t\t);
\t\t\t\tINFOPLIST_KEY_NSHumanReadableCopyright = "";
\t\t\t\tLD_RUNPATH_SEARCH_PATHS = (
\t\t\t\t\t"@executable_path/../Frameworks",
\t\t\t\t);
\t\t\t\tLIBRARY_SEARCH_PATHS = (
\t\t\t\t\t"\$(SRCROOT)/lib",
\t\t\t\t\t"\$(SRCROOT)/target/release",
\t\t\t\t\t"\$(SRCROOT)/target/debug",
\t\t\t\t);
\t\t\t\tMARKETING_VERSION = 1.0;
\t\t\t\tOTHER_LDFLAGS = (
\t\t\t\t\t"-lairgap_sync",
\t\t\t\t\t"-lresolv",
\t\t\t\t);
\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = #{BUNDLE_ID};
\t\t\t\tPRODUCT_NAME = "\$(TARGET_NAME)";
\t\t\t\tSWIFT_EMIT_LOC_STRINGS = YES;
\t\t\t\tSWIFT_OBJC_BRIDGING_HEADER = "\$(SRCROOT)/AirGapSync/AirGapSync-Bridging-Header.h";
\t\t\t\tSWIFT_VERSION = 5.0;
\t\t\t};
\t\t\tname = Debug;
\t\t};
\t\t#{uuid} /* Release */ = {
\t\t\tisa = XCBuildConfiguration;
\t\t\tbuildSettings = {
\t\t\t\tASSETCATALOG_COMPILER_APPICON_NAME = AppIcon;
\t\t\t\tCODE_SIGN_STYLE = Automatic;
\t\t\t\tCOMBINE_HIDPI_IMAGES = YES;
\t\t\t\tCURRENT_PROJECT_VERSION = 1;
\t\t\t\tDEVELOPMENT_TEAM = "";
\t\t\t\tENABLE_HARDENED_RUNTIME = YES;
\t\t\t\tENABLE_PREVIEWS = YES;
\t\t\t\tGENERATE_INFOPLIST_FILE = YES;
\t\t\t\tHEADER_SEARCH_PATHS = (
\t\t\t\t\t"\$(SRCROOT)/lib",
\t\t\t\t\t"\$(SRCROOT)/target",
\t\t\t\t);
\t\t\t\tINFOPLIST_KEY_NSHumanReadableCopyright = "";
\t\t\t\tLD_RUNPATH_SEARCH_PATHS = (
\t\t\t\t\t"@executable_path/../Frameworks",
\t\t\t\t);
\t\t\t\tLIBRARY_SEARCH_PATHS = (
\t\t\t\t\t"\$(SRCROOT)/lib",
\t\t\t\t\t"\$(SRCROOT)/target/release",
\t\t\t\t\t"\$(SRCROOT)/target/debug",
\t\t\t\t);
\t\t\t\tMARKETING_VERSION = 1.0;
\t\t\t\tOTHER_LDFLAGS = (
\t\t\t\t\t"-lairgap_sync",
\t\t\t\t\t"-lresolv",
\t\t\t\t);
\t\t\t\tPRODUCT_BUNDLE_IDENTIFIER = #{BUNDLE_ID};
\t\t\t\tPRODUCT_NAME = "\$(TARGET_NAME)";
\t\t\t\tSWIFT_EMIT_LOC_STRINGS = YES;
\t\t\t\tSWIFT_OBJC_BRIDGING_HEADER = "\$(SRCROOT)/AirGapSync/AirGapSync-Bridging-Header.h";
\t\t\t\tSWIFT_VERSION = 5.0;
\t\t\t};
\t\t\tname = Release;
\t\t};
/* End XCBuildConfiguration section */

/* Begin XCConfigurationList section */
\t\t#{config_list_project_uuid} /* Build configuration list for PBXProject "#{PROJECT_NAME}" */ = {
\t\t\tisa = XCConfigurationList;
\t\t\tbuildConfigurations = (
\t\t\t\t#{build_config_debug_uuid} /* Debug */,
\t\t\t\t#{build_config_release_uuid} /* Release */,
\t\t\t);
\t\t\tdefaultConfigurationIsVisible = 0;
\t\t\tdefaultConfigurationName = Release;
\t\t};
\t\t#{config_list_target_uuid} /* Build configuration list for PBXNativeTarget "#{PROJECT_NAME}" */ = {
\t\t\tisa = XCConfigurationList;
\t\t\tbuildConfigurations = (
\t\t\t\t#{uuid} /* Debug */,
\t\t\t\t#{uuid} /* Release */,
\t\t\t);
\t\t\tdefaultConfigurationIsVisible = 0;
\t\t\tdefaultConfigurationName = Release;
\t\t};
/* End XCConfigurationList section */
\t};
\trootObject = #{project_uuid} /* Project object */;
}
PBXPROJ

# Write the pbxproj file
File.write("#{PROJECT_DIR}/#{PROJECT_NAME}.xcodeproj/project.pbxproj", pbxproj)

puts "✅ Xcode project created successfully!"
puts ""
puts "Project: #{PROJECT_DIR}/#{PROJECT_NAME}.xcodeproj"
puts "Configuration:"
puts "  - Rust library will build automatically before Swift compilation"
puts "  - FFI bridging header configured"
puts "  - Library search paths configured"
puts "  - Linker flags configured"
puts ""
puts "Next steps:"
puts "  1. Open project: open #{PROJECT_NAME}.xcodeproj"
puts "  2. Build: Cmd+B"
puts "  3. Run: Cmd+R"
puts ""
puts "Note: First build may take several minutes while Rust compiles."
