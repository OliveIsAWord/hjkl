.POSIX:
.SUFFIXES: .o .asm .jkl

OUT = hjkl.fxf
OUT_IMG = hjkl.img
OBJECTS = hjkl.o os.o rom.o
SOURCE = $(OBJECTS:.o=.jkl)

$(OUT): $(OBJECTS)
	target=fox32 $(XRLINK) link $(OUT) $(OBJECTS) $(RTLLIB)

.jkl.asm:
	$(JACKAL) target=fox32 $< $@

.asm.o:
	$(XRASM) target=fox32 $< $@

$(OUT_IMG): $(OUT)
	$(RYFS) create $(OUT_IMG) -l Hjkl -s 0
	$(RYFS) add $(OUT_IMG) $(OUT)

run: $(OUT_IMG)
	$(FOX32) --disk $(FOX32OS) --disk $(OUT_IMG)

clean:
	rm -f $(OUT) $(OUT_IMG) $(OBJECTS)

.PHONY: clean run
