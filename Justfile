build:
    cargo build --target x86_64-unknown-uefi

run:
	cp target/x86_64-unknown-uefi/debug/nocciolo.efi esp/efi/boot/bootx64.efi
	qemu-system-x86_64 \
		-drive if=pflash,format=raw,readonly=on,file=target/OVMF_CODE.fd \
		-drive if=pflash,format=raw,readonly=on,file=target/OVMF_VARS.fd \
		-drive format=raw,file=fat:rw:esp

install-apt:
	sudo apt install qemu ovmf

init:
	rustup target add x86_64-unknown-uefi
	mkdir -p esp/efi/boot
	cp /usr/share/OVMF/OVMF_CODE.fd target
	cp /usr/share/OVMF/OVMF_VARS.fd target
