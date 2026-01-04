function toggleDrawer() {
    const drawer = document.getElementById('drawer');
    const backdrop = document.getElementById('drawerBackdrop');

    drawer.classList.toggle('open');
    backdrop.classList.toggle('visible');
}

function closeDrawer() {
    const drawer = document.getElementById('drawer');
    const backdrop = document.getElementById('drawerBackdrop');

    drawer.classList.remove('open');
    backdrop.classList.remove('visible');
}
