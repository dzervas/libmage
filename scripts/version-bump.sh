#!/bin/bash

function parse_version() {
	local version="$1"

	major=$(echo -n "$version" | cut -d. -f1 || exit 1)
	minor=$(echo -n "$version" | cut -d. -f2 || exit 1)
	patch=$(echo -n "$version" | cut -d. -f3 || exit 1)
}

function bump_version() {
	local version="$1"
	# One of: major, minor, patch
	local action="$2"

	parse_version "$version"

	if [ "$action" == "major" ]; then
		new_major=$(( major + 1 ))
		new_minor="0"
		new_patch="0"
	elif [ "$action" == "minor" ]; then
		new_major="$major"
		new_minor=$(( minor + 1 ))
		new_patch="0"
	elif [ "$action" == "patch" ]; then
		new_major="$major"
		new_minor="$minor"
		new_patch=$(( patch + 1 ))
	else
		echo "[bump_version] Could not understand action = '$action' - should be one of: major, minor, patch"
		exit 1
	fi

	new_version="$new_major.$new_minor.$new_patch"
}

pushd $(git rev-parse --show-toplevel)

# Action can be: patch, minor, major or <version-number>
# version number is in the form of 1.2.3
if [ "$1" == "-h" ] || [ "$1" == "--help" ]; then
	echo "Usage: $0 [-h|--help] [patch|minor|major|<version-number>]"
	echo
	echo "Examples:"
	echo -e "\t$0  # Will bump 1.2.3 -> 1.2.4"
	echo -e "\t$0 patch  # Will bump 1.2.3 -> 1.2.4"
	echo -e "\t$0 minor  # 1.2.3 -> 1.3.0"
	echo -e "\t$0 major  # 1.2.3 -> 2.0.0"
	echo -e "\t$0 2.5.50  # 1.2.3 -> 2.5.50"

	exit 0
elif [ "$1" == "patch" ] || [ "$1" == "minor" ] || [ "$1" == "major" ]; then
	action="$1"
elif [ -z "$1" ]; then
	action="patch"
else
	action="version"
	new_version="$1"

	parse_version "$new_version"

	new_major="$major"
	new_minor="$minor"
	new_patch="$patch"
fi

old_cargo_version=$(grep -E '^version = "[0-9]+\.[0-9]+\.[0-9]+"$' Cargo.toml | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')
echo "Detected Cargo.toml version: $old_cargo_version"

if [ -z "$new_version" ]; then
	bump_version "$old_cargo_version" "$action"
fi

echo "Bumping Cargo.toml to version $new_version"
sed -i"" "s/^version = \"$old_cargo_version\"$/version = \"$new_version\"/" Cargo.toml

popd
