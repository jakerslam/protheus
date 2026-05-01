function infringTaskbarClockParts(page) {
  var tick = Number(page.clockTick || Date.now());
  var dt = new Date(tick);
  if (!Number.isFinite(dt.getTime())) dt = new Date();
  var dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  var monthNames = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  var dayName = dayNames[dt.getDay()] || '';
  var monthName = monthNames[dt.getMonth()] || '';
  var day = dt.getDate();
  var hours24 = dt.getHours();
  var minutes = dt.getMinutes();
  var suffix = hours24 >= 12 ? 'PM' : 'AM';
  var hours12 = hours24 % 12;
  if (hours12 === 0) hours12 = 12;
  var minuteText = minutes < 10 ? ('0' + minutes) : String(minutes);
  return {
    main: dayName + ' ' + monthName + ' ' + day + ' ' + hours12 + ':' + minuteText,
    meridiem: suffix
  };
}

function infringTaskbarClockMainLabel(page) {
  return page.taskbarClockParts().main;
}

function infringTaskbarClockMeridiemLabel(page) {
  return page.taskbarClockParts().meridiem;
}

function infringTaskbarClockLabel(page) {
  var parts = page.taskbarClockParts();
  return parts.main + ' ' + parts.meridiem;
}
