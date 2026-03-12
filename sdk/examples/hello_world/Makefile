# PaperDOS App Build - Hello World
#
# Prerequisites:
#   - riscv32-esp-elf-gcc (from ESP-IDF or standalone RISC-V toolchain)
#   - Python 3
#
# Usage:
#   make            # Build hello.pdb
#   make validate   # Validate the built .pdb file
#   make clean      # Remove build artifacts
#   make install    # Copy to mounted SD card (set SD_MOUNT)

# ─── Toolchain ───
CC      = riscv32-esp-elf-gcc
OBJCOPY = riscv32-esp-elf-objcopy
PYTHON  = python3

# ─── Paths ───
SDK_DIR    = ../..
LINKER_DIR = $(SDK_DIR)/linker
TOOLS_DIR  = $(SDK_DIR)/tools
INCLUDE    = $(SDK_DIR)/include

# ─── App metadata ───
APP_NAME    = Hello World
APP_VERSION = 1.0.0
ABI_VERSION = 1

# ─── SD card mount point (for `make install`) ───
SD_MOUNT ?= /mnt/sdcard

# ─── Build flags ───
CFLAGS  = -march=rv32imc -mabi=ilp32 -Os -nostdlib -ffreestanding
CFLAGS += -I$(INCLUDE)
CFLAGS += -Wall -Wextra -Wno-unused-parameter
LDFLAGS = -T $(LINKER_DIR)/paperdos.ld -nostdlib

# ─── Source files ───
SRCS    = $(LINKER_DIR)/pd_entry.S main.c
TARGET  = hello

# ─── Rules ───
.PHONY: all clean validate install

all: $(TARGET).pdb

$(TARGET).elf: $(SRCS)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^

$(TARGET).pdb: $(TARGET).elf
	$(PYTHON) $(TOOLS_DIR)/pdpack.py $< $@ \
		--name "$(APP_NAME)" \
		--version "$(APP_VERSION)" \
		--abi $(ABI_VERSION)

validate: $(TARGET).pdb
	$(PYTHON) $(TOOLS_DIR)/pdpack.py $< /dev/null --validate

install: $(TARGET).pdb
	@echo "Copying $(TARGET).pdb to $(SD_MOUNT)/paperdos/apps/"
	@mkdir -p $(SD_MOUNT)/paperdos/apps
	cp $(TARGET).pdb $(SD_MOUNT)/paperdos/apps/

clean:
	rm -f $(TARGET).elf $(TARGET).pdb
