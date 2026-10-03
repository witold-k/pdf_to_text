# PDF backend installation

`pdf_to_text` can use GROBID and MinerU as PDF extraction backends. This document describes reproducible installation and startup. Machine-specific paths belong in `~/.config/pdf_to_text/config.json`, not in this file.

## MinerU

Source: https://github.com/opendatalab/MinerU

MinerU 4 requires Python >= 3.10 and < 3.15. A simple local installation uses `uv` and a dedicated virtual environment:

```bash
mkdir -p ~/opt/services/mineru
cd ~/opt/services/mineru
uv venv --python 3.12 .venv
source .venv/bin/activate
uv pip install -U "mineru>=4.0,<5"
```

This installs the MinerU executables inside:

```text
~/opt/services/mineru/.venv/bin/
```

For example, the stateless batch parser is `mineru-kit`. Start the local V1 API service with:

```bash
~/opt/services/mineru/.venv/bin/mineru-kit api-server --host 127.0.0.1 --port 8000 --tier standard
```

The API documentation is then available at `http://127.0.0.1:8000/docs`.

For `pdf_to_text`, use the actual executable path from the virtual environment in the local config rather than relying on the shell's activated environment.

## GROBID

Source: https://github.com/grobidOrg/grobid

GROBID is installed natively from source. Building the current GROBID source requires OpenJDK 21.

A simple installation under `~/opt` is:

```bash
mkdir -p ~/opt/services
cd ~/opt/services
git clone https://github.com/grobidOrg/grobid.git
cd grobid
./gradlew clean assemble
```

The installation/source directory is then:

```text
~/opt/services/grobid
```

Start the GROBID service directly from the source tree with:

```bash
cd ~/opt/services/grobid
./gradlew :grobid-service:run
```

The service is then reachable at:

```text
http://localhost:8070
```

and `pdf_to_text` uses the full-text endpoint:

```text
http://localhost:8070/api/processFulltextDocument
```

For `pdf_to_text`, configure the native GROBID start command/path in the local configuration. No Docker installation is required.

## Local configuration

On first use, `pdf_to_text` creates:

```text
~/.config/pdf_to_text/config.json
```

The configuration contains the backend sources, executable paths, service commands/arguments, and GROBID URL. Installation locations and other machine-specific changes should be recorded there.

The commands and version examples in this document follow the upstream documentation at the time they were added. When upgrading MinerU or GROBID, check the source links above for current installation and service options.
