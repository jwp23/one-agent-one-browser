#include <stdint.h>
#include <wayland-client-protocol.h>

#include "xdg-shell-client-protocol.h"

struct wl_registry *oab_wl_display_get_registry(struct wl_display *display) {
    return wl_display_get_registry(display);
}

struct wl_compositor *oab_wl_registry_bind_compositor(struct wl_registry *registry, uint32_t name, uint32_t version) {
    return wl_registry_bind(registry, name, &wl_compositor_interface, version);
}

struct wl_shm *oab_wl_registry_bind_shm(struct wl_registry *registry, uint32_t name, uint32_t version) {
    return wl_registry_bind(registry, name, &wl_shm_interface, version);
}

struct wl_seat *oab_wl_registry_bind_seat(struct wl_registry *registry, uint32_t name, uint32_t version) {
    return wl_registry_bind(registry, name, &wl_seat_interface, version);
}

struct xdg_wm_base *oab_wl_registry_bind_xdg_wm_base(struct wl_registry *registry, uint32_t name, uint32_t version) {
    return wl_registry_bind(registry, name, &xdg_wm_base_interface, version);
}

struct wl_surface *oab_wl_compositor_create_surface(struct wl_compositor *compositor) {
    return wl_compositor_create_surface(compositor);
}

struct wl_shm_pool *oab_wl_shm_create_pool(struct wl_shm *shm, int32_t fd, int32_t size) {
    return wl_shm_create_pool(shm, fd, size);
}

struct wl_buffer *oab_wl_shm_pool_create_buffer(
    struct wl_shm_pool *pool,
    int32_t offset,
    int32_t width,
    int32_t height,
    int32_t stride,
    uint32_t format
) {
    return wl_shm_pool_create_buffer(pool, offset, width, height, stride, format);
}

void oab_wl_shm_pool_destroy(struct wl_shm_pool *pool) {
    wl_shm_pool_destroy(pool);
}

void oab_wl_buffer_destroy(struct wl_buffer *buffer) {
    wl_buffer_destroy(buffer);
}

void oab_wl_surface_attach(struct wl_surface *surface, struct wl_buffer *buffer, int32_t x, int32_t y) {
    wl_surface_attach(surface, buffer, x, y);
}

void oab_wl_surface_damage_buffer(struct wl_surface *surface, int32_t x, int32_t y, int32_t width, int32_t height) {
    wl_surface_damage_buffer(surface, x, y, width, height);
}

struct wl_callback *oab_wl_surface_frame(struct wl_surface *surface) {
    return wl_surface_frame(surface);
}

void oab_wl_surface_set_buffer_scale(struct wl_surface *surface, int32_t scale) {
    wl_surface_set_buffer_scale(surface, scale);
}

void oab_wl_surface_commit(struct wl_surface *surface) {
    wl_surface_commit(surface);
}

void oab_wl_surface_destroy(struct wl_surface *surface) {
    wl_surface_destroy(surface);
}

struct wl_pointer *oab_wl_seat_get_pointer(struct wl_seat *seat) {
    return wl_seat_get_pointer(seat);
}

void oab_wl_pointer_release(struct wl_pointer *pointer) {
    wl_pointer_release(pointer);
}

void oab_wl_seat_release(struct wl_seat *seat) {
    wl_seat_release(seat);
}

void oab_wl_shm_release(struct wl_shm *shm) {
    wl_shm_release(shm);
}

struct xdg_surface *oab_xdg_wm_base_get_xdg_surface(struct xdg_wm_base *wm_base, struct wl_surface *surface) {
    return xdg_wm_base_get_xdg_surface(wm_base, surface);
}

void oab_xdg_wm_base_pong(struct xdg_wm_base *wm_base, uint32_t serial) {
    xdg_wm_base_pong(wm_base, serial);
}

void oab_xdg_wm_base_destroy(struct xdg_wm_base *wm_base) {
    xdg_wm_base_destroy(wm_base);
}

struct xdg_toplevel *oab_xdg_surface_get_toplevel(struct xdg_surface *surface) {
    return xdg_surface_get_toplevel(surface);
}

void oab_xdg_surface_ack_configure(struct xdg_surface *surface, uint32_t serial) {
    xdg_surface_ack_configure(surface, serial);
}

void oab_xdg_surface_destroy(struct xdg_surface *surface) {
    xdg_surface_destroy(surface);
}

void oab_xdg_toplevel_set_title(struct xdg_toplevel *toplevel, const char *title) {
    xdg_toplevel_set_title(toplevel, title);
}

void oab_xdg_toplevel_set_app_id(struct xdg_toplevel *toplevel, const char *app_id) {
    xdg_toplevel_set_app_id(toplevel, app_id);
}

void oab_xdg_toplevel_destroy(struct xdg_toplevel *toplevel) {
    xdg_toplevel_destroy(toplevel);
}
