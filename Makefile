.POSIX:
.SUFFIXES: .o .asm .jkl

OUT = hjkl.fxf
OUT_IMG = hjkl.img
OBJECT = hjkl.o
ASM = hjkl.asm
SOURCE = src/build.jkl

$(OUT): $(SOURCE)
	$(JACKAL) target=fox32 $(SOURCE) $(ASM)
	$(XRASM) target=fox32 $(ASM) $(OBJECT)
	target=fox32 $(XRLINK) link $(OUT) $(OBJECT) $(RTLLIB)

$(OUT_IMG): $(OUT)
	$(RYFS) create $(OUT_IMG) -l Hjkl -s 0
	$(RYFS) add $(OUT_IMG) $(OUT)

run: $(OUT_IMG)
	$(FOX32) --disk $(FOX32OS) --disk $(OUT_IMG)

dogfood_build: $(OUT_IMG)
	mkdir -p backup
	cp -r src "backup/$$(date --rfc-3339=seconds)"
	cd src && ls
	for FILE in $$(ls src/*.jkl src/*.hjk); do \
		$(RYFS) add $(OUT_IMG) $$FILE; \
	done

dogfood_export:
	cd src && \
	for FILE in $$(ls *.jkl *.hjk); do \
		../$(RYFS) export ../$(OUT_IMG) $$FILE; \
	done

dogfood: dogfood_build run dogfood_export

clean:
	rm -f $(OUT) $(OUT_IMG) $(OBJECT) $(ASM)

.PHONY: clean run dogfood_build dogfood dogfood_export
.NOTPARALLEL: dogfood
