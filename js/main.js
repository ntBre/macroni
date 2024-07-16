async function deleteRow(rowid) {
	if (!confirm("Delete this food?")) {
		return;
	}
	let response = await fetch(`/delete-food?id=${rowid}`, {
		method: "POST"
	});
	if (!response.ok) {
		console.log(response);
		return;
	}
	window.location.reload();
}

async function addToday(rowid, date) {
	let response = await fetch(`/add-food-today?id=${rowid}&date=${date}`, {
		method: "POST"
	});
	if (!response.ok) {
		console.log(response);
		return;
	}
	window.location.reload();
}
