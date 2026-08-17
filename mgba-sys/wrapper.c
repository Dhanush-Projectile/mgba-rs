#include <mgba/core/core.h>

void wrapper_mCoreInit(struct mCore* core) {
    core->init(core);
}

void wrapper_mCoreDeinit(struct mCore* core) {
    core->deinit(core);
}

void wrapper_mCoreReset(struct mCore* core) {
    core->reset(core);
}

void wrapper_mCoreRunFrame(struct mCore* core) {
    core->runFrame(core);
}

void wrapper_mCoreSetVideoBuffer(struct mCore* core, mColor* buffer, size_t stride) {
    core->setVideoBuffer(core, buffer, stride);
}

void wrapper_mCoreSetKeys(struct mCore* core, uint32_t keys) {
    core->setKeys(core, keys);
}
