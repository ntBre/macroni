.PHONY: screenshot.png doc

%.png:
	sleep 5
	maim -i $$(xdotool getactivewindow) $@

doc:
	cargo doc --open

run:
	cargo run 2> log
