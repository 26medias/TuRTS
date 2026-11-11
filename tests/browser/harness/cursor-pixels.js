// turts-test-role: harness
"use strict";

const assert = require("node:assert/strict");

function parseOctal(bytes, start, end, label) {
    const value = bytes.toString("ascii", start, end).replace(/[\0 ]+$/u, "").trim();
    assert(/^[0-7]+$/u.test(value), `invalid tar ${label} field`);
    const parsed = Number.parseInt(value, 8);
    assert(Number.isSafeInteger(parsed) && parsed >= 0, `invalid tar ${label} value`);
    return parsed;
}

function readUstarFiles(input) {
    const archive = Buffer.from(input);
    const files = new Map();
    for (let offset = 0; offset + 512 <= archive.length;) {
        const header = archive.subarray(offset, offset + 512);
        if (header.every((byte) => byte === 0)) break;
        assert.equal(header.toString("ascii", 257, 263), "ustar\0", "cursor assets require a USTAR package archive");
        const expectedChecksum = parseOctal(header, 148, 156, "checksum");
        let checksum = 0;
        for (let index = 0; index < 512; index += 1) {
            checksum += index >= 148 && index < 156 ? 32 : header[index];
        }
        assert.equal(checksum, expectedChecksum, "package archive tar checksum mismatch");
        const field = (start, end) => header.toString("utf8", start, end).replace(/\0.*$/su, "");
        const name = field(0, 100);
        const prefix = field(345, 500);
        const archivePath = prefix ? `${prefix}/${name}` : name;
        assert(archivePath && !archivePath.startsWith("/") && !archivePath.split("/").includes(".."),
            "package archive contains an unsafe path");
        const type = header[156];
        const size = parseOctal(header, 124, 136, "size");
        const dataStart = offset + 512;
        const dataEnd = dataStart + size;
        assert(dataEnd <= archive.length, "package archive file is truncated");
        if (type === 0 || type === 48) {
            assert(!files.has(archivePath), `package archive duplicates ${archivePath}`);
            files.set(archivePath, archive.subarray(dataStart, dataEnd));
        } else {
            assert(type === 53 && size === 0, "cursor asset archive contains an unsupported non-file entry");
        }
        offset = dataStart + Math.ceil(size / 512) * 512;
    }
    return files;
}

function readAuthoredCursorAssets(archiveBytes, decodePng) {
    assert.equal(typeof decodePng, "function", "packed cursor decoding requires the existing screenshot PNG decoder");
    const files = readUstarFiles(archiveBytes);
    const ronBytes = files.get("assets/assets.ron");
    assert(ronBytes, "actual package archive lacks assets/assets.ron");
    const ron = ronBytes.toString("utf8");
    const imagesSection = ron.match(/\bimages:\s*\[([\s\S]*?)\],\s*fonts:/u)?.[1];
    const cursorsSection = ron.match(/\bcursors:\s*\[([\s\S]*?)\]\s*\)/u)?.[1];
    assert(imagesSection && cursorsSection, "actual package manifest lacks image or cursor declarations");
    const imagePaths = new Map();
    for (const match of imagesSection.matchAll(/\(\s*id:\s*"([^"]+)"\s*,\s*path:\s*"([^"]+)"[^)]*\)/gu)) {
        assert(!imagePaths.has(match[1]), `actual package declares duplicate image id ${match[1]}`);
        imagePaths.set(match[1], match[2]);
    }
    const cursors = new Map();
    for (const match of cursorsSection.matchAll(/\(\s*id:\s*"([^"]+)"\s*,\s*image:\s*"([^"]+)"\s*,\s*hotspot:\s*\(\s*(\d+)\s*,\s*(\d+)\s*\)\s*\)/gu)) {
        const [, id, imageId, xText, yText] = match;
        const hotspot = [Number(xText), Number(yText)];
        assert(!cursors.has(id), `actual package declares duplicate cursor id ${id}`);
        assert(hotspot.every((value) => Number.isSafeInteger(value) && value >= 0 && value < 32),
            `actual package cursor ${id} has an unsupported hotspot`);
        const imagePath = imagePaths.get(imageId);
        assert(imagePath && /^assets\/images\/cursor_[^/]+\.png$/u.test(imagePath),
            `actual package cursor ${id} does not reference an authored cursor PNG`);
        const imageBytes = files.get(imagePath);
        assert(imageBytes, `actual package archive lacks ${imagePath}`);
        const image = decodePng(imageBytes);
        assert.equal(image.width, 32, `actual package cursor ${imageId} has unexpected width`);
        assert.equal(image.height, 32, `actual package cursor ${imageId} has unexpected height`);
        cursors.set(id, { image, hotspot, image_path: imagePath });
    }
    assert(cursors.size > 0, "actual package manifest contains no authored cursor declarations");
    return cursors;
}

function templateMatch(image, asset, origin) {
    let best = { ratio: 0, matched: 0, total: 0, offset: [0, 0] };
    for (const dx of [-1, 0, 1]) {
        for (const dy of [-1, 0, 1]) {
            let total = 0;
            let matched = 0;
            for (let y = 0; y < asset.height; y += 1) {
                for (let x = 0; x < asset.width; x += 1) {
                    const assetIndex = (y * asset.width + x) * 4;
                    if (asset.rgba[assetIndex + 3] < 250 || Math.max(asset.rgba[assetIndex], asset.rgba[assetIndex + 1], asset.rgba[assetIndex + 2]) < 100) continue;
                    const imageX = Math.round(origin[0]) + x + dx;
                    const imageY = Math.round(origin[1]) + y + dy;
                    if (imageX < 0 || imageY < 0 || imageX >= image.width || imageY >= image.height) continue;
                    total += 1;
                    const imageIndex = (imageY * image.width + imageX) * 4;
                    if (image.rgba[imageIndex + 3] >= 250 && [0, 1, 2].every((channel) =>
                        Math.abs(image.rgba[imageIndex + channel] - asset.rgba[assetIndex + channel]) <= 12)) matched += 1;
                }
            }
            const ratio = total ? matched / total : 0;
            if (ratio > best.ratio || (ratio === best.ratio && total > best.total)) best = { ratio, matched, total, offset: [dx, dy] };
        }
    }
    return best;
}

function measureCursorPixels(first, last, firstImage, lastImage, cursors) {
    assert(first.window === last.window && first.document_epoch === last.document_epoch && first.input_call < last.input_call,
        "cursor captures must be ordered in the same document and window");
    const stages = [];
    for (const [capture, image] of [[first, firstImage], [last, lastImage]]) {
        assert(capture.checkpoint.window === capture.window && capture.checkpoint.document_epoch === capture.document_epoch &&
            capture.checkpoint.input_call === capture.input_call, "cursor checkpoint identity mismatch");
        assert(image.width === 1280 && image.height === 720, "cursor calibration requires an actual 1280x720 PNG");
        assert.equal(capture.checkpoint.scene.scale_factor, 1, "cursor calibration requires actual DPR 1");
        assert.deepEqual(capture.checkpoint.scene.viewport_physical_rect, [0, 0, 1280, 720]);
        const cursor = capture.checkpoint.debug.cursor;
        assert(cursor?.lock_active === true && cursor.lock_requested === true && capture.checkpoint.browser?.canvas_pointer_locked === true,
            "cursor pixels require actual acquired canvas lock");
        assert(Array.isArray(cursor.engine_position) && cursor.engine_position.length === 2 && cursor.engine_position.every(Number.isFinite),
            "actual engine pointer position is unavailable");
        const entry = cursors.get(cursor.image_id);
        assert(entry, `actual package has no cursor asset declaration for ${cursor.image_id}`);
        const origin = cursor.engine_position.map((value, index) => value - entry.hotspot[index]);
        stages.push({ capture, image, cursor, entry, origin, positive: templateMatch(image, entry.image, origin) });
    }
    assert(Math.hypot(stages[0].cursor.engine_position[0] - stages[1].cursor.engine_position[0],
        stages[0].cursor.engine_position[1] - stages[1].cursor.engine_position[1]) >= 64,
    "cursor pixel pair needs separated actual engine positions");
    for (const stage of stages) {
        const [x, y] = stage.origin;
        const nearbyX = x + 80 < stage.image.width ? x + 48 : x - 48;
        assert(nearbyX >= 0 && nearbyX + 32 <= stage.image.width, "cursor nearby absence control falls outside the PNG");
        stage.nearby = templateMatch(stage.image, stage.entry.image, [nearbyX, y]);
    }
    const absence = templateMatch(lastImage, stages[0].entry.image, stages[0].origin);
    return {
        stages: stages.map(({ capture, cursor, origin, positive, nearby }) => ({
            input_call: capture.input_call,
            window: capture.window,
            document_epoch: capture.document_epoch,
            image_id: cursor.image_id,
            engine_position: cursor.engine_position,
            hotspot: cursor.image_id ? cursors.get(cursor.image_id).hotspot : null,
            origin,
            positive,
            nearby,
        })),
        old_region_absence: absence,
    };
}

module.exports = { readUstarFiles, readAuthoredCursorAssets, templateMatch, measureCursorPixels };
