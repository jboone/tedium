/*
 * This file is part of Tedium.
 *
 * Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
 * SPDX-License-Identifier: BSD-3-Clause
 */

#include <cstdio>
#include <cstring>
#include <unistd.h>
#include <byteswap.h>

#include <libusb-1.0/libusb.h>

const uint16_t VENDOR_ID  = 0x16d0;
const uint16_t PRODUCT_ID = 0x0f3b;

const uint8_t INTERRUPT_ENDPOINT_NUMBER = 9;

const uint8_t ISO_OUT_ENDPOINT_NUMBER = 1;
const uint8_t ISO_OUT_ENDPOINT_ADDRESS = ISO_OUT_ENDPOINT_NUMBER | LIBUSB_ENDPOINT_OUT;
const uint8_t ISO_OUT_INTERFACE = 1;
const unsigned int ISO_OUT_TIMEOUT = 1000;
const int NUM_ISO_OUT_PACKETS = 1;

const uint8_t ISO_IN_ENDPOINT_NUMBER = 2;
const uint8_t ISO_IN_ENDPOINT_ADDRESS = ISO_IN_ENDPOINT_NUMBER | LIBUSB_ENDPOINT_IN;
const uint8_t ISO_IN_INTERFACE = 2;
const unsigned int ISO_IN_TIMEOUT = 1000;
const int NUM_ISO_IN_PACKETS = 16;

// It seems that having a lot of ISO packets in reserve helps avoid dropped bits.
// I'm still not sure what cranking up the ISO packets gets you vs. an increased
// number of transfers.
const size_t NUM_TRANSFERS = 8 * 10;

const size_t FRAME_LENGTH = 24;
const size_t TRANSFER_LENGTH = FRAME_LENGTH * 2;

static FILE* f_out = NULL;
static uint8_t last = 0;

static uint32_t expected_data_frame_count = 0;
static uint32_t expected_framer_data = 0;


static FILE* f_ulaw_in = NULL;

// bool open_audio_input_file() {
// 	if( f_ulaw_in != NULL) {
// 		fclose(f_ulaw_in);
// 		f_ulaw_in = NULL;
// 	}

// 	f_ulaw_in = fopen("/home/jboone/src/tedium/example/audio/092393_hell_01_ITR.au", "rb");

// 	return f_ulaw_in != NULL;
// }

void callback_iso_in(libusb_transfer* transfer) {
	// printf("IN: callback: %d: %d\n", transfer->status, transfer->iso_packet_desc[0].actual_length);

	// "If this is an isochronous transfer, this field may read COMPLETED even if there were errors in the frames. Use the status field in each packet to determine if errors occurred."
	if( transfer->status == LIBUSB_TRANSFER_COMPLETED ) {

		for(auto i=0; i<NUM_ISO_IN_PACKETS; i++) {
			auto packet = &transfer->iso_packet_desc[i];

			if( packet->status == LIBUSB_TRANSFER_COMPLETED) {
				if( packet->actual_length > 0 ) {
					if( (packet->actual_length % FRAME_LENGTH) == 0 ) {
						auto b = libusb_get_iso_packet_buffer_simple(transfer, i);

						const bool usb_iso_rx_debug = false;

						if( usb_iso_rx_debug ) {
							for(auto j=0; j<packet->actual_length; j+=FRAME_LENGTH) {
								auto c = &b[j];
								const uint32_t framer_data      = *((uint32_t *)&c[ 0]);
								const uint32_t usb_clock_count  = bswap_32(*((uint32_t *)&c[ 4]));
								const uint32_t usb_frame_count  = bswap_32(*((uint32_t *)&c[ 8]));
								const uint32_t data_frame_count = bswap_32(*((uint32_t *)&c[12]));
								const uint32_t fifo_r_level     = bswap_32(*((uint32_t *)&c[16]));

								// printf("IN: %08x\n", data_frame_count);

								if( expected_data_frame_count != data_frame_count ) {
									printf("IN: data frame: expected %08x, got %08x\n", expected_data_frame_count, data_frame_count);
								}
								expected_data_frame_count = data_frame_count + 1;

								// if( expected_framer_data != framer_data ) {
								// 	printf("IN: framer data: expected %08x, got %08x\n", expected_framer_data, framer_data);
								// }
								// expected_framer_data = framer_data + 1;
							}
						} else {

						}

						fwrite(b, packet->actual_length, 1, f_out);

						// printf(".");
					} else {
						printf("IN: packet %d incomplete, length %d\n", i, packet->actual_length);
					}
				// } else {
				// 	printf("IN: packet %d length = %d\n", i, packet->actual_length);
				}
			} else {
				printf("IN: packet %d status = %d\n", i, packet->status);
			}
		}
	} else {
		printf("IN: transfer status = %d\n", transfer->status);
	}

	auto result = libusb_submit_transfer(transfer);
	if( result != 0 ) {
		printf("IN: libusb_submit_transfer failed: %d\n", result);
	}
}

void callback_iso_out(libusb_transfer* transfer) {
	// printf("OUT: callback: %d: %d\n", transfer->status, transfer->iso_packet_desc[0].actual_length);

	libusb_set_iso_packet_lengths(transfer, 24);

	for(auto i=0; i<NUM_ISO_OUT_PACKETS; i++) {
		auto packet = &transfer->iso_packet_desc[i];
		auto b = libusb_get_iso_packet_buffer_simple(transfer, i);

		uint8_t v = 0xff;
		if( fread(&v, sizeof(v), 1, f_ulaw_in) == 0 ) {
			if( feof(f_ulaw_in) ) {
				if( fseek(f_ulaw_in, 0, SEEK_SET) == 0 ) {
					fread(&v, sizeof(v), 1, f_ulaw_in);
				} else {
					f_ulaw_in = NULL;
				}
			} else {
				f_ulaw_in = NULL;
			}
			// open_audio_input_file();

			// fread(&v, sizeof(v), 1, f_ulaw_in);
		}

		memset(b, v, packet->length);

		// printf("OUT: packet[%d] length=%d actual_length=%d\n", i, packet->length, packet->actual_length);
	}

	auto result = libusb_submit_transfer(transfer);
	if( result != 0 ) {
		printf("OUT: libusb_submit_transfer failed: %d\n", result);
	}
}

int main(int argc, char ** argv) {
	f_ulaw_in = fopen("/home/jboone/src/tedium/example/audio/092393_hell_01_ITR.au", "rb");
	if( f_ulaw_in == NULL ) {
		printf("fopen(in) failed\n");
		return -1;
	}

	f_out = fopen("/tmp/bob_c.u8", "wb");
	if( f_out == NULL ) {
		printf("fopen failed\n");
		return -1;
	}

	libusb_context* context = NULL;
	if( libusb_init(&context) != 0 ) {
		printf("libusb_init failed\n");
		return -2;
	}

	auto device_handle = libusb_open_device_with_vid_pid(context, VENDOR_ID, PRODUCT_ID);
	if( device_handle == NULL ) {
		printf("libusb_open_device_with_vid_pid failed\n");
		return -3;
	}

	auto result = libusb_claim_interface(device_handle, ISO_IN_INTERFACE);
	if( result != 0 ) {
		printf("IN: libusb_claim_interface failed: %d\n", result);
		return -4;
	}

	result = libusb_set_interface_alt_setting(device_handle, ISO_IN_INTERFACE, 1);
	if( result != 0 ) {
		printf("IN: libusb_set_interface_alt_setting: %d\n", result);
		return -5;
	}

	// auto device = libusb_get_device(device_handle);

	// auto packet_size = libusb_get_max_iso_packet_size(device, ISO_IN_ENDPOINT_ADDRESS);
	// printf("packet size: %d\n", packet_size);

	// ISO IN transfers
	for(auto i=0; i<NUM_TRANSFERS; i++) {
		auto transfer = libusb_alloc_transfer(NUM_ISO_IN_PACKETS);
		if( transfer == NULL ) {
			printf("IN: libusb_alloc_transfer failed\n");
			return -6;
		}

		const size_t BLOCK_LENGTH = TRANSFER_LENGTH;
		const size_t BUFFER_LENGTH = BLOCK_LENGTH * NUM_ISO_IN_PACKETS;
		auto buffer = new uint8_t[BUFFER_LENGTH];
		libusb_fill_iso_transfer(transfer, device_handle, ISO_IN_ENDPOINT_ADDRESS, buffer, BUFFER_LENGTH, NUM_ISO_IN_PACKETS, callback_iso_in, NULL, ISO_IN_TIMEOUT);

		libusb_set_iso_packet_lengths(transfer, BLOCK_LENGTH);

		result = libusb_submit_transfer(transfer);
		if( result != 0 ) {
			printf("IN: libusb_submit_transfer failed: %d\n", result);
			return -7;
		}
	}

	result = libusb_claim_interface(device_handle, ISO_OUT_INTERFACE);
	if( result != 0 ) {
		printf("OUT: libusb_claim_interface failed: %d\n", result);
		return -8;
	}

	result = libusb_set_interface_alt_setting(device_handle, ISO_OUT_INTERFACE, 1);
	if( result != 0 ) {
		printf("OUT: libusb_set_interface_alt_setting: %d\n", result);
		return -9;
	}

	// ISO OUT transfers
	for(auto i=0; i<NUM_TRANSFERS; i++) {
		auto transfer = libusb_alloc_transfer(NUM_ISO_OUT_PACKETS);
		if( transfer == NULL ) {
			printf("OUT: libusb_alloc_transfer failed\n");
			return -10;
		}

		const size_t BLOCK_LENGTH = TRANSFER_LENGTH;
		const size_t BUFFER_LENGTH = BLOCK_LENGTH * NUM_ISO_OUT_PACKETS;
		auto buffer = new uint8_t[BUFFER_LENGTH];
		libusb_fill_iso_transfer(transfer, device_handle, ISO_OUT_ENDPOINT_ADDRESS, buffer, BUFFER_LENGTH, NUM_ISO_OUT_PACKETS, callback_iso_out, NULL, ISO_OUT_TIMEOUT);

		libusb_set_iso_packet_lengths(transfer, 24);

		for(auto i=0; i<NUM_ISO_OUT_PACKETS; i++) {
			auto packet = &transfer->iso_packet_desc[i];

			printf("OUT: packet[%d] length=%d actual_length=%d\n", i, packet->length, packet->actual_length);
		}

		// transfer->iso_packet_desc[0].length = 24;

		result = libusb_submit_transfer(transfer);
		if( result != 0 ) {
			printf("OUT: libusb_submit_transfer failed: %d\n", result);
			return -11;
		}
	}

	// printf("I'm alive!\n");
	// printf("...but sleeping.\n");

	while(true) {
		result = libusb_handle_events(context);
		if( result != 0 ) {
			printf("libusb_handle_events failed: %d\n", result);
			return -12;
		}
	}
	// usleep(3000000);

	libusb_exit(context);

	return 0;
}
