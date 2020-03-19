#!/usr/bin/env bash
# Exit script on any error
set -e

REQUIRED_DIRS=(api db scripts)
BUILD_DIR="target/debug/"
REPORT_DIR="report/"

source_root_dir="bn-api"



echo "Checking the current execution path":
BASE_PATH=$(pwd)
for CHECK in ${REQUIRED_DIRS}
do
    if [[ ! -d "${BASE_PATH}/${CHECK}" ]]; then
        echo "Could not find ${BASE_PATH}/${CHECK}, please start this script from the root project directory"
        exit 1;
    fi
done

echo -n "Check if grcov installed:"
if [[ -z $(which grcov) ]]
then
    echo " Installing ..."
    cargo install grcov
else
    echo " Installed"
fi

echo -n "Check if lcov installed:"
if [[ -z $(which lcov) ]]
then
    echo " Installing..."
    if [[ -z $(which brew) ]];
    then
        echo "Please install Homebrew: "
        echo ruby -e \"\$\(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/master/install\)\"
        exit 1
    fi
    brew install lcov
else
    echo " Installed"
fi

echo -n "Clear Build Directory:"
if [[ -d ${BUILD_DIR} ]]; then
    rm -rf ${BUILD_DIR}
    echo " Removed"
else
    echo " Directory Cleared"
fi

echo -n "Clear Report Directory:"
if [[ -d ${REPORT_DIR} ]]; then
    rm -rf ${REPORT_DIR}
    echo " Removed"
else
    echo " Directory Cleared"
fi

#Make clean directories for Build and Report
mkdir -p ${BUILD_DIR}
mkdir -p ${REPORT_DIR}



echo "Build project.."
export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Cinline-threshold=0 -Clink-dead-code -Coverflow-checks=off -Zno-landing-pads"
cargo +nightly build --verbose ${CARGO_OPTIONS}

# Undo exiting the script on non-zero result because a test may fail
set +e
echo "Perform project Tests.."
cargo +nightly test --verbose ${CARGO_OPTIONS}
set -e

echo "Acquire all build and test files for coverage check.."
CCOV_FILENAME="ccov.zip"
CCOV_PATH="${REPORT_DIR}${CCOV_FILENAME}"
zip -0 ${CCOV_PATH} `find ${BUILD_DIR} \( -name "api*.gc*" \) -print`;

echo "Perform grcov code coverage.."
LCOV_FILENAME="lcov.info"
LCOV_PATH="${REPORT_DIR}${LCOV_FILENAME}"
grcov "$CCOV_PATH" -s . -t lcov --llvm --branch --ignore-not-existing --ignore-dir "/*" > "$LCOV_PATH";

echo "Generate report from code coverage.."
LOCAL_LOCOV_PATH="${REPORT_DIR}${LCOV_FILENAME}"
genhtml -o ${REPORT_DIR} --show-details --highlight --legend ${LOCAL_LOCOV_PATH}

echo "Launch report in browser.."
INDEX_FILE="index.html"
open "${REPORT_DIR}${INDEX_FILE}"
