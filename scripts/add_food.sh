#!/bin/bash

curl -X POST '0.0.0.0:3333/add-food' \
	 -d "name=bread&calories=123&carbs=29&fat=2&protein=3&unit=slice"
