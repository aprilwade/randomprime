
__attribute__((visibility("hidden")))
int bcmp(const unsigned char *b1, const unsigned char *b2, unsigned long len)
{
    if (len == 0) {
        return 0;
    }

    while (len > 0) {
        if (*b1 != *b2) {
            break;
        }

        b1++;
        b2++;
        len--;
    }

    return len;
}
