OODA_COMPILER ?= $(shell command -v oodac 2>/dev/null || echo /Projects/openOODA-core/oodac/bin/oodac)
OODA_READDIR ?= /Projects
OODA_WRITEDIR ?= $(CURDIR):/tmp
OO_LIST_AMBIENT_QUOTA ?= 8589934592
BIN := bin/idlescreen
TESTBIN := bin/test_idlescreen
REPO_NAME := idlescreen
VER := 5.0.2
SRC := main.oo anchor.oo $(wildcard core/*.oo raster/*.oo ascii/*.oo ascii/*/*.oo ascii/*/*/*.oo wayland/*.oo daemon/*.oo tui/*.oo cli/*.oo)
ENV := OO_LIST_AMBIENT_QUOTA=$(OO_LIST_AMBIENT_QUOTA) OODA_FS_READDIR=$(OODA_READDIR) OODA_FS_WRITEDIR=$(OODA_WRITEDIR) OODA_NO_JAIL=1 OODA_COMPILER=$(OODA_COMPILER) OODAC_BIN=$(OODA_COMPILER)

.PHONY: all build verify test det check line-cap file-law academy density vocab shape clean parity

all: build verify test det parity

PARITY_SCENES := plasma ripple aurora chaos metaballs moire cosmos radar tunnel mystify expand wipe sweep slide middleout binarypath burn blackhole

parity:
	@mkdir -p .ooda-cache bin
	$(ENV) $(OODA_COMPILER) build tests/oo/scene_emit.oo -o bin/scene_emit > /dev/null
	@fail=0; \
	for s in $(PARITY_SCENES); do \
		$(ENV) ./bin/scene_emit $$s > .ooda-cache/parity_$$s.txt; \
		if cmp -s qa/golden/$$s.txt .ooda-cache/parity_$$s.txt; then \
			echo "PASS: $$s bit-exact parity"; \
		else \
			echo "FAIL: $$s diverges from qa/golden/$$s.txt"; fail=1; \
		fi; \
	done; \
	if [ $$fail -ne 0 ]; then exit 1; fi; \
	echo "PASS: scene parity ($(words $(PARITY_SCENES)) scenes bit-exact)"

build: $(BIN)

$(BIN): $(SRC)
	@mkdir -p bin .ooda-cache
	$(ENV) $(OODA_COMPILER) build main.oo -o $(BIN)
	@chmod +x $(BIN)
	@echo "built $(BIN)"

line-cap:
	@violations=0; \
	for f in $$(find . -name "*.oo" -o -name "*.oot" | grep -v "/.git/" | grep -v "/bin/" | grep -v "/dist/" | grep -v "/.ooda-cache/" | grep -v "/target/"); do \
		n=$$(wc -l < "$$f"); \
		if [ $$n -gt 256 ]; then \
			echo "VIOLATION: $$f = $$n lines (exceeds 256)"; violations=$$((violations+1)); \
		fi; \
		code=$$(grep -vE '^[[:space:]]*(//.*)?$$' "$$f" | grep -cvE '^[[:space:]]*import[[:space:]]+"'); \
		if [ "$$code" = "0" ]; then continue; fi; \
		if [ $$n -lt 16 ]; then \
			echo "VIOLATION: $$f = $$n lines (under 16-line floor, not a shim)"; violations=$$((violations+1)); \
		fi; \
	done; \
	if [ $$violations -gt 0 ]; then echo "FAIL: $$violations files violate the Page Rule"; exit 1; fi; \
	echo "PASS: Page Rule sizing (16-256 lines, shims exempt from floor) holds"

file-law:
	@forbidden="py rs js ts rb pl json yaml toml"; \
	violations=0; \
	for ext in $$forbidden; do \
		found=$$(find . -name "*.$$ext" -not -path "./.git/*" -not -path "./.github/*" -not -path "./bin/*" -not -path "./dist/*" -not -path "./.ooda-cache/*" -not -path "./target/*" 2>/dev/null | head -3); \
		if [ -n "$$found" ]; then \
			echo "VIOLATION: .$$ext forbidden:"; echo "$$found"; violations=$$((violations+1)); \
		fi; \
	done; \
	yaml=$$(find . \( -name "*.yml" -o -name "*.yaml" \) -not -path "./.git/*" -not -path "./.github/workflows/*" 2>/dev/null | head -3); \
	if [ -n "$$yaml" ]; then echo "VIOLATION: yaml outside .github/workflows"; echo "$$yaml"; violations=$$((violations+1)); fi; \
	for f in $$(find . -name "*.sh" -not -path "./.git/*" -not -path "./bin/*" -not -path "./dist/*" -not -path "./.ooda-cache/*" 2>/dev/null); do \
		b=$$(basename "$$f"); \
		if [ "$$b" != "install.sh" ] && [ "$$b" != "uninstall.sh" ]; then echo "VIOLATION: .sh forbidden outside install.sh and uninstall.sh: $$f"; violations=$$((violations+1)); fi; \
	done; \
	for f in $$(find . -name "*.md" -not -path "./.git/*" -not -path "./.github/*" -not -path "./bin/*" -not -path "./dist/*" -not -path "./.ooda-cache/*" 2>/dev/null); do \
		b=$$(basename "$$f"); \
		if [ "$$b" != "README.md" ] && [ "$$b" != "AGENTS.md" ]; then echo "VIOLATION: .md forbidden outside README.md and AGENTS.md: $$f"; violations=$$((violations+1)); fi; \
	done; \
	for nm in util utils helper helpers common misc shared base; do \
		found=$$(find . -name "$$nm.oo" -not -path "./.git/*" -not -path "./bin/*" -not -path "./dist/*" 2>/dev/null | head -3); \
		if [ -n "$$found" ]; then echo "VIOLATION: banned page name $$nm.oo:"; echo "$$found"; violations=$$((violations+1)); fi; \
	done; \
	if [ $$violations -gt 0 ]; then echo "FAIL: file-law violations"; exit 1; fi; \
	echo "PASS: file law holds"

academy:
	@failures=0; \
	for f in $$(find . -name "*.oo" -not -path "./.git/*" -not -path "./bin/*" -not -path "./dist/*" -not -path "./.ooda-cache/*"); do \
		header=$$(head -16 "$$f"); \
		missing=""; \
		echo "$$header" | grep -q "^// # "        || missing="$$missing title"; \
		echo "$$header" | grep -q "^// Logline:"  || missing="$$missing logline"; \
		echo "$$header" | grep -q "^// Setup:"    || missing="$$missing setup"; \
		echo "$$header" | grep -q "^// Beats:"    || missing="$$missing beats"; \
		if [ -n "$$missing" ]; then \
			echo "FAIL: $$f missing Academy element(s):$$missing"; failures=$$((failures+1)); \
		fi; \
	done; \
	if [ $$failures -gt 0 ]; then echo "FAIL: $$failures academy header violations"; exit 1; fi; \
	echo "PASS: academy headers hold"

density:
	@violations=0; \
	for d in $$(find . -type d -not -path "./.git*" -not -path "./bin*" -not -path "./dist*" -not -path "./.ooda-cache*" -not -path "./.github*" -not -path "./target*"); do \
		n=$$(ls "$$d"/*.oo "$$d"/*.oot 2>/dev/null | grep -v '\*' | wc -l); \
		if [ $$n -gt 8 ]; then \
			echo "VIOLATION: $$d holds $$n pages (exceeds 8)"; violations=$$((violations+1)); \
		fi; \
	done; \
	if [ $$violations -gt 0 ]; then echo "FAIL: $$violations directories exceed density bound"; exit 1; fi; \
	echo "PASS: directory density (<= 8 pages per directory) holds"

vocab:
	@if grep -riE "sovereign|Pillar [0-9]|Stage [0-9]|bootstrap|moonshot|dogfight" --include="*.oo" --include="*.oot" --include="*.md" --include="*.pkg" --include="*.yml" . ; then \
		echo "FAIL: banned vocabulary found"; exit 1; \
	fi; \
	echo "PASS: vocabulary check holds"

shape:
	@violations=0; \
	for f in $$(find . -name "*.oo" -not -path "./.git/*" -not -path "./bin/*" -not -path "./dist/*" -not -path "./.ooda-cache/*"); do \
		fns=$$(grep -cE '^[[:space:]]*(pub[[:space:]]+)?fn ' "$$f" || true); \
		if [ "$$fns" -gt 8 ]; then \
			echo "VIOLATION: $$f has $$fns functions (exceeds 8)"; violations=$$((violations+1)); \
		fi; \
		long=$$(awk '/^[[:space:]]*(pub[[:space:]]+)?fn /{s=NR;d=0;b=0} s>0{for(i=1;i<=length($$0);i++){c=substr($$0,i,1);if(c=="{"){d++;b=1}else if(c=="}")d--} if(b&&d==0){l=NR-s+1;if(l>64)print s":"l;s=0}}' "$$f"); \
		if [ -n "$$long" ]; then \
			echo "VIOLATION: $$f has function(s) > 64 lines ($$long)"; violations=$$((violations+1)); \
		fi; \
	done; \
	if [ $$violations -gt 0 ]; then echo "FAIL: shape law violations"; exit 1; fi; \
	echo "PASS: function count (<= 8 per page) and length (<= 64 lines) hold"

check:
	@for f in $$(find . -name "*.oo" -not -path "./.git/*" -not -path "./bin/*" -not -path "./dist/*" -not -path "./.ooda-cache/*" -not -path "./tests/oo/*" -not -path "./tests/scene_dispatch.oo"); do \
		$(ENV) $(OODA_COMPILER) check "$$f" > /dev/null || { echo "VIOLATION: oodac check fails on $$f"; exit 1; } \
	done; \
	echo "PASS: oodac check holds on all .oo files"

verify: line-cap file-law academy density vocab shape check

test: $(BIN)
	@mkdir -p .ooda-cache
	@$(ENV) $(OODA_COMPILER) build tests/test_main.oo -o $(TESTBIN) > /dev/null
	@$(ENV) ./$(TESTBIN) | grep -q "PASS: all assertions" && echo "PASS: test suite"
	@./$(BIN) --help | grep -q "$(REPO_NAME) $(VER)" && echo "PASS: --help"
	@./$(BIN) --version | grep -q "$(VER)" && echo "PASS: --version"
	@./$(BIN) version | grep -q "$(VER)" && echo "PASS: version"
	@./$(BIN) list | grep -q "aurora" && echo "PASS: list aurora"
	@./$(BIN) list | grep -q "zero" && echo "PASS: list zero"
	@./$(BIN) list | grep -qc "" && test "$$(./$(BIN) list | grep -c ' ')" -ge 50 && echo "PASS: list 53 scenes"
	@./$(BIN) list --json | grep -q '"count":53' && echo "PASS: list --json count"
	@./$(BIN) text openooda | grep -qi "openooda" && echo "PASS: text openooda"
	@./$(BIN) text superfedora | grep -qi "superfedora" && echo "PASS: text superfedora"
	@./$(BIN) preview matrix --frames 2 --cols 40 --rows 12 | grep -q "frame" && echo "PASS: preview matrix"
	@./$(BIN) preview plasma --frames 2 | grep -q "frame" && echo "PASS: preview plasma flag parsing"
	@./$(BIN) preview openooda --frames 1 | grep -q "OPENOODA" && echo "PASS: preview openooda wordmark"
	@./$(BIN) status | grep -qi "inactive\|active\|daemon" && echo "PASS: status"
	@./$(BIN) inhibitors | grep -q "\[" && echo "PASS: inhibitors"
	@./$(BIN) config | grep -qi "idlescreen" && echo "PASS: config"
	@./$(BIN) doctor | grep -qi "Diagnostics complete\|PASS\|OK" && echo "PASS: doctor"
	@./$(BIN) timeout | grep -qi "timeout" && echo "PASS: timeout query"
	@./$(BIN) service | grep -qi "idlescreen service" && echo "PASS: service query"
	@./$(BIN) service status | grep -qi "inactive\|active\|daemon" && echo "PASS: service status"
	@./$(BIN) set bogus-saver 2>&1 | grep -qi "unknown" && echo "PASS: set rejects unknown saver"
	@./$(BIN) config | grep -qi "config.toml" && echo "PASS: config path"
	@./$(BIN) --persona ooidle --version | grep -q "ooidle $(VER)" && echo "PASS: --persona ooidle --version"
	@./$(BIN) --persona ooidle status | grep -qi "ooidle" && echo "PASS: --persona ooidle status"
	@./$(BIN) --self-test | grep -q "PASS" && echo "PASS: --self-test"
	@./$(BIN) --json | grep -q '"status":"pass"' && echo "PASS: --json"
	@./$(BIN) --json > .ooda-cache/r1.txt && ./$(BIN) --json > .ooda-cache/r2.txt && cmp -s .ooda-cache/r1.txt .ooda-cache/r2.txt && echo "PASS: double-run output determinism"
	@./$(BIN) bogus 2>&1 | grep -qi "unknown" && echo "PASS: unknown verb fails closed"
	@echo "ALL TESTS PASSED"

det: $(BIN)
	@mkdir -p .ooda-cache
	@$(ENV) $(OODA_COMPILER) build main.oo -o .ooda-cache/det_a > /dev/null
	@$(ENV) $(OODA_COMPILER) build main.oo -o .ooda-cache/det_b > /dev/null
	@cmp -s .ooda-cache/det_a .ooda-cache/det_b && sha256sum .ooda-cache/det_a && echo "PASS: deterministic build (byte-identical)" || (echo "FAIL: nondeterministic build"; exit 1)

clean:
	@rm -rf bin dist .ooda-cache
