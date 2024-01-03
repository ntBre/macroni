.PHONY: screenshot.png doc

screenshot.png:
	sleep 5
	maim -i $$(xdotool getactivewindow) $@

doc:
	cargo doc --open
