# Shared Android SDK paths for bash scripts. Source from repo scripts only.
inertia_repo_root() {
	cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd
}

inertia_android_sdk_root() {
	if [[ -n "${ANDROID_HOME:-}" ]]; then
		echo "$ANDROID_HOME"
	elif [[ -n "${ANDROID_SDK_ROOT:-}" ]]; then
		echo "$ANDROID_SDK_ROOT"
	else
		echo "$HOME/Android/Sdk"
	fi
}

inertia_android_adb() {
	local sdk
	sdk="$(inertia_android_sdk_root)"
	echo "$sdk/platform-tools/adb"
}

inertia_require_adb() {
	local adb
	adb="$(inertia_android_adb)"
	if [[ ! -x "$adb" ]]; then
		echo "adb not found at $adb — run: npm run android:sdk" >&2
		exit 1
	fi
}

inertia_java_home() {
	local candidate ver

	if [[ -n "${JAVA_HOME:-}" && -x "${JAVA_HOME}/bin/java" ]]; then
		ver="$("${JAVA_HOME}/bin/java" -version 2>&1 | head -1)"
		if [[ "$ver" =~ version\ \"(1\.[89]|2[01])\. ]]; then
			echo "$JAVA_HOME"
			return 0
		fi
	fi

	for candidate in \
		/usr/lib/jvm/java-21-openjdk \
		/usr/lib/jvm/java-17-openjdk \
		/opt/android-studio/jbr \
		"$HOME/android-studio/jbr"; do
		if [[ -x "$candidate/bin/java" ]]; then
			echo "$candidate"
			return 0
		fi
	done

	if command -v flatpak >/dev/null 2>&1 && flatpak info com.google.AndroidStudio >/dev/null 2>&1; then
		candidate="$(flatpak info --show-location com.google.AndroidStudio)/files/extra/jbr"
		if [[ -x "$candidate/bin/java" ]]; then
			echo "$candidate"
			return 0
		fi
	fi

	echo 'JDK 17 or 21 required (Gradle does not support Java 25). Install java-21-openjdk or Android Studio.' >&2
	return 1
}
