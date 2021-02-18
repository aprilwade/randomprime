#include "squish.h"

extern "C" {

// void libsquish_CompressMasked(u8 const* rgba, int mask, void* block, int flags, float* metric) {
//     squish::CompressMasked(rgba, mask, block, flags, metric);
// }
void libsquish_Compress(const squish::u8* rgba, void* block, int flags, float* metric)  {
     squish::Compress(rgba, block, flags, metric);
}
void libsquish_Decompress(squish::u8* rgba, const void* block, int flags) {
    squish::Decompress(rgba, block, flags);
}
// int libsquish_GetStorageRequirements(int width, int height, int flags) {
//     squish::GetStorageRequirements(width, height, flags);
// }

// void libsquish_CompressImage(u8 const* rgba, int width, int height, void* blocks, int flags, float* metric) {
//     squish::CompressImage(rgba, width, height, blocks, flags, metric);
// }

// void libsquish_DecompressImage(u8* rgba, int width, int height, void const* blocks, int flags) {
//     squish::DecompressImage(rgba, width, height, blocks, flags);
// }

}
