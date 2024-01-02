.PHONY: screenshot.png

screenshot.png:
	sleep 5
	maim -i $$(xdotool getactivewindow) $@
