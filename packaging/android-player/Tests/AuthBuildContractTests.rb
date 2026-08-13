# frozen_string_literal: true

player_dir = File.expand_path("..", __dir__)
repo_dir = File.expand_path("../..", player_dir)
gradle = File.read(File.join(player_dir, "app/build.gradle.kts"))
gitignore = File.read(File.join(player_dir, ".gitignore"))
readme = File.read(File.join(player_dir, "README.md"))
script = File.read(File.join(repo_dir, "scripts/build-mobile-auth-dev.sh"))

raise "main jniLibs must be explicitly empty" unless gradle.include?('sourceSets["main"].jniLibs.setSrcDirs(emptyList<String>())')
raise "Debug jniLibs source set missing" unless gradle.include?('sourceSets["debug"].jniLibs.srcDirs("src/debug/jniLibs")')
raise "Release jniLibs source set missing" unless gradle.include?('sourceSets["release"].jniLibs.srcDirs("src/release/jniLibs")')
raise "Debug native output must stay untracked" unless gitignore.include?("app/src/debug/jniLibs/")
raise "Release native output must stay untracked" unless gitignore.include?("app/src/release/jniLibs/")
raise "auth build script must target Debug jniLibs" unless script.include?("app/src/debug/jniLibs")
raise "auth build script must never request Cargo release" if script.include?("--release")
raise "auth build script must use the explicit feature" unless script.include?("mobile-auth-dev")
raise "README must document Release source isolation" unless readme.include?("app/src/release/jniLibs")
raise "README must state that no unsigned auth ships" unless readme.include?("cannot leak into Release")

puts "Android auth build contract validates"
