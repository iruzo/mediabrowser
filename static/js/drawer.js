function toggleDrawer() {
    const drawer = document.getElementById('drawer');
    const backdrop = document.getElementById('drawerBackdrop');

    drawer.classList.toggle('open');
    backdrop.classList.toggle('visible');
}
