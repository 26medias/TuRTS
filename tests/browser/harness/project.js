// turts-test-role: driver
"use strict";

function basis(camera) {
    const horizontal = camera.height * Math.tan(camera.pitch);
    const eye = [
        camera.lookAt[0] + Math.sin(camera.yaw) * horizontal,
        camera.height,
        camera.lookAt[1] + Math.cos(camera.yaw) * horizontal,
    ];
    const forward = normalize([camera.lookAt[0] - eye[0], -eye[1], camera.lookAt[1] - eye[2]]);
    const right = normalize(cross(forward, [0, 1, 0]));
    const up = cross(right, forward);
    return { eye, forward, right, up };
}

function projectWorldPoint(point, camera, viewport) {
    if (!pair(point) || !Number.isFinite(viewport.width) || !Number.isFinite(viewport.height) || viewport.width <= 0 || viewport.height <= 0) return null;
    const { eye, forward, right, up } = basis(camera);
    const delta = [point[0] - eye[0], -eye[1], point[1] - eye[2]];
    const z = dot(delta, forward);
    if (z <= 0) return null;
    const tan = Math.tan(camera.fov / 2);
    const x = dot(delta, right) / (z * tan * viewport.width / viewport.height);
    const y = dot(delta, up) / (z * tan);
    const px = (x + 1) * viewport.width / 2;
    const py = (1 - y) * viewport.height / 2;
    return px >= 0 && py >= 0 && px < viewport.width && py < viewport.height ? [px, py] : null;
}

function screenToWorldPoint(point, camera, viewport) {
    if (!pair(point) || !Number.isFinite(viewport.width) || !Number.isFinite(viewport.height) || viewport.width <= 0 || viewport.height <= 0) return null;
    const { eye, forward, right, up } = basis(camera);
    const nx = point[0] / viewport.width * 2 - 1;
    const ny = 1 - point[1] / viewport.height * 2;
    const tan = Math.tan(camera.fov / 2);
    const ray = normalize(add(forward, add(scale(right, nx * tan * viewport.width / viewport.height), scale(up, ny * tan))));
    if (Math.abs(ray[1]) < 1e-9) return null;
    const t = -eye[1] / ray[1];
    if (t < 0) return null;
    const world = add(eye, scale(ray, t));
    return [world[0], world[2]];
}

function pair(value) { return Array.isArray(value) && value.length === 2 && value.every(Number.isFinite); }
function dot(a, b) { return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]; }
function cross(a, b) { return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]; }
function add(a, b) { return [a[0] + b[0], a[1] + b[1], a[2] + b[2]]; }
function scale(a, n) { return [a[0] * n, a[1] * n, a[2] * n]; }
function normalize(a) { const length = Math.hypot(...a); return length > 0 ? scale(a, 1 / length) : [0, 0, 0]; }

module.exports = { projectWorldPoint, screenToWorldPoint };
