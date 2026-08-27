#include <mgba/core/core.h>
#include <mgba/core/interface.h>
#include <mgba/core/config.h>
#include <mgba/core/log.h>
#include <mgba/core/version.h>
#include <mgba/gba/core.h>
#include <mgba/gba/interface.h>
#include <mgba/internal/gba/input.h>
#include <mgba-util/vfs.h>
#include <mgba-util/audio-buffer.h>

void wrapper_mCoreInit(struct mCore* core);
void wrapper_mCoreDeinit(struct mCore* core);
void wrapper_mCoreReset(struct mCore* core);
void wrapper_mCoreRunFrame(struct mCore* core);
void wrapper_mCoreSetVideoBuffer(struct mCore* core, mColor* buffer, size_t stride);
void wrapper_mCoreSetKeys(struct mCore* core, uint32_t keys);
struct mAudioBuffer* wrapper_mCoreGetAudioBuffer(struct mCore* core);
unsigned wrapper_mCoreAudioSampleRate(const struct mCore* core);
uint32_t wrapper_mCoreBusRead16(struct mCore* core, uint32_t address);
void wrapper_mCoreSetOptionVolume(struct mCore* core, int volume);
bool wrapper_mCoreLoadSave(struct mCore* core, const char* path);
